//! Encrypted-message I/O for `WebRtcManager` (Task 15 + Task 19.1).
//!
//! This module hosts the four entry points that move application
//! payloads through the AES-GCM envelope path:
//!
//! * [`WebRtcManager::send_encrypted_message`] — low-level, takes a
//!   pre-framed `[discriminator][bitcode]` byte slice.
//! * [`WebRtcManager::send_encrypted_data_channel_message`] — high
//!   level, accepts a typed `DataChannelMessage` and frames it for
//!   you (preferred).
//! * [`WebRtcManager::broadcast_encrypted_message`] — fan-out over
//!   every peer that has derived a shared key.
//! * [`WebRtcManager::receive_encrypted_message`] — decrypt a raw
//!   ciphertext from an inbound envelope frame.
//!
//! Plus the two read-only helpers [`has_encryption_key`] and
//! [`encrypted_peers`] used by callers to short-circuit when no keys
//! exist yet.
//!
//! Extracted from `webrtc/mod.rs` so the main module stays below the
//! 2 000-line hotspot threshold (Task 15 T15-1 refactor).

use super::{BroadcastResult, WebRtcError, WebRtcManager};
use message::UserId;
use message::error::{ErrorCategory, ErrorCode, ErrorModule};

impl WebRtcManager {
  /// Send an encrypted message to a specific peer.
  ///
  /// Encrypts the plaintext with the peer's shared key and sends it
  /// wrapped in an `ENCRYPTED_MARKER` envelope frame.
  ///
  /// # Errors
  /// Returns an error if the peer has no shared key or the send
  /// operation fails.
  pub async fn send_encrypted_message(
    &self,
    peer_id: UserId,
    plaintext: &[u8],
  ) -> Result<(), WebRtcError> {
    // Scope borrow to extract crypto.
    let crypto = {
      let inner = self.inner.borrow();
      inner
        .crypto
        .get(&peer_id)
        .ok_or_else(|| WebRtcError::no_crypto(peer_id.clone()))?
        .clone()
    };

    if !crypto.has_shared_key() {
      return Err(WebRtcError::no_shared_key(peer_id.clone()));
    }

    let encrypted = crypto.encrypt(plaintext).await.map_err(|e| {
      WebRtcError::new(
        ErrorCode::new(ErrorModule::Cht, ErrorCategory::Security, 1),
        format!("Encryption failed: {}", e),
        Some(peer_id.clone()),
      )
    })?;

    // Extract DataChannel (clone to avoid holding RefCell borrow across send).
    let dc = {
      let inner = self.inner.borrow();
      let pc = inner
        .connections
        .get(&peer_id)
        .ok_or_else(|| WebRtcError::peer_not_found(peer_id.clone()))?;

      pc.get_data_channel().cloned().ok_or_else(|| {
        WebRtcError::new(
          ErrorCode::new(ErrorModule::E2e, ErrorCategory::Client, 2),
          "No DataChannel for peer",
          Some(peer_id.clone()),
        )
      })?
    };

    dc.send_raw_envelope(&encrypted).map_err(|e| {
      WebRtcError::new(
        ErrorCode::new(ErrorModule::Cht, ErrorCategory::Network, 1),
        format!("DataChannel send failed: {}", e),
        Some(peer_id.clone()),
      )
    })
  }

  /// Encode, encrypt and dispatch a [`DataChannelMessage`] via the
  /// envelope path (Task 19.1 — Req 5.1.3).
  ///
  /// This is the preferred send entry point for every application-data
  /// message (chat, file, media control). It:
  ///
  /// 1. Serialises the message with `bitcode` and prepends its
  ///    `discriminator` byte to produce a plaintext frame.
  /// 2. Encrypts the plaintext with the peer's shared AES-GCM key
  ///    (fails fast with `WebRtcError::no_shared_key` when ECDH has
  ///    not completed yet, so the caller can retry after backoff).
  /// 3. Wraps the ciphertext in an `ENCRYPTED_MARKER` envelope and
  ///    ships it through `PeerDataChannel::send_raw_envelope`.
  ///
  /// Callers that still need the raw `Vec<u8>` plaintext form (e.g.
  /// tests, the chat housekeeping layer) can fall back to
  /// [`send_encrypted_message`](Self::send_encrypted_message) instead.
  ///
  /// # Errors
  /// Returns an error when no shared key has been derived for the
  /// peer, when the DataChannel is not open, or when the crypto /
  /// send operation fails.
  pub async fn send_encrypted_data_channel_message(
    &self,
    peer_id: UserId,
    msg: &message::datachannel::DataChannelMessage,
  ) -> Result<(), WebRtcError> {
    let discriminator = msg.discriminator();
    let payload = bitcode::encode(msg);
    let mut plaintext = Vec::with_capacity(1 + payload.len());
    plaintext.push(discriminator);
    plaintext.extend_from_slice(&payload);
    self.send_encrypted_message(peer_id, &plaintext).await
  }

  /// Broadcast an encrypted message to all peers with established keys.
  ///
  /// Encrypts the plaintext individually for each peer and sends it over
  /// their respective DataChannels. Partial failures are collected in
  /// [`BroadcastResult::failed_peers`] so callers (e.g. chat UI) can
  /// display per-peer delivery status (P1-17 fix).
  ///
  /// # ⚠️ Input contract (Task 19.1 D-1)
  ///
  /// `plaintext` must already be framed as
  /// `[DataChannelMessage::discriminator (1 B)][bitcode payload]`.
  /// The envelope receive path unwraps the decrypted bytes under the
  /// same layout and a mismatching first byte will be dropped with a
  /// `Discriminator mismatch` warning.
  ///
  /// Prefer [`send_encrypted_data_channel_message`](Self::send_encrypted_data_channel_message)
  /// for a single peer or fan out over
  /// [`encrypted_peers`](Self::encrypted_peers) yourself when you
  /// have a typed `DataChannelMessage` — those entry points build
  /// the frame for you. This lower-level API is retained for
  /// low-level tests and rare callers that have already built the
  /// framed plaintext.
  ///
  /// # Errors
  /// Returns an error if no peers have encryption keys.
  ///
  /// # Returns
  /// [`BroadcastResult`] with the count of successful sends and per-peer
  /// failure details.
  pub async fn broadcast_encrypted_message(
    &self,
    plaintext: &[u8],
  ) -> Result<BroadcastResult, WebRtcError> {
    let peers = self.encrypted_peers();
    if peers.is_empty() {
      return Err(WebRtcError::new(
        ErrorCode::new(ErrorModule::E2e, ErrorCategory::Client, 5),
        "No peers with established encryption keys",
        None,
      ));
    }

    // P1-9 fix: fan out encryption + send across all peers concurrently.
    // Each peer gets its own pairwise-encrypted copy (Req 5.2.10);
    // independent futures let the JS event loop interleave AES-GCM
    // and `send()` per peer instead of awaiting them sequentially.
    let futures = peers
      .iter()
      .map(|peer_id| self.send_encrypted_message(peer_id.clone(), plaintext));
    let results = futures::future::join_all(futures).await;

    let mut sent = 0;
    let mut failed_peers = Vec::new();
    for (idx, result) in results.into_iter().enumerate() {
      match result {
        Ok(()) => sent += 1,
        Err(e) => {
          web_sys::console::warn_1(&format!("[webrtc] Broadcast to peer failed: {e}").into());
          failed_peers.push((peers[idx].clone(), e));
        }
      }
    }
    Ok(BroadcastResult { sent, failed_peers })
  }

  /// Receive and decrypt a message from a peer.
  ///
  /// Decrypts ciphertext using the peer's established shared key.
  ///
  /// # Errors
  /// Returns an error if no shared key exists or decryption fails.
  pub async fn receive_encrypted_message(
    &self,
    peer_id: UserId,
    ciphertext: &[u8],
  ) -> Result<Vec<u8>, WebRtcError> {
    // Scope borrow to extract crypto.
    let crypto = {
      let inner = self.inner.borrow();
      inner
        .crypto
        .get(&peer_id)
        .ok_or_else(|| WebRtcError::no_crypto(peer_id.clone()))?
        .clone()
    };

    if !crypto.has_shared_key() {
      return Err(WebRtcError::no_shared_key(peer_id.clone()));
    }

    crypto.decrypt(ciphertext).await.map_err(|e| {
      WebRtcError::new(
        ErrorCode::new(ErrorModule::Cht, ErrorCategory::Security, 2),
        format!("Decryption failed: {}", e),
        Some(peer_id.clone()),
      )
    })
  }

  /// Check if a peer has an established shared encryption key.
  #[must_use]
  pub fn has_encryption_key(&self, peer_id: &UserId) -> bool {
    self
      .inner
      .borrow()
      .crypto
      .get(peer_id)
      .is_some_and(|c| c.has_shared_key())
  }

  /// Get the list of peers with established encryption keys.
  #[must_use]
  pub fn encrypted_peers(&self) -> Vec<UserId> {
    self
      .inner
      .borrow()
      .crypto
      .iter()
      .filter(|(_, c)| c.has_shared_key())
      .map(|(id, _)| id.clone())
      .collect()
  }
}
