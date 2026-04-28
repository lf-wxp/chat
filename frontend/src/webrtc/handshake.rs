//! ECDH handshake path for [`WebRtcManager`] (split out of `mod.rs`).
//!
//! This module owns every code path related to the Diffie-Hellman key
//! agreement that establishes a per-peer AES-GCM key. Nothing here
//! touches application-level framing; for that, see [`super::raw_frame`]
//! and [`super::crypto_ops`].
//!
//! # Responsibilities
//!
//! * [`WebRtcManager::handle_ecdh_key`] — process an inbound
//!   `EcdhKeyExchange`, import the peer's public key, and mirror the
//!   completion into reactive UI state.
//! * [`WebRtcManager::send_datachannel_ecdh_key_direct`] — push a
//!   locally-generated public key over an already-open DataChannel.
//! * [`WebRtcManager::buffer_pending_ecdh_key`] — stash a key while
//!   the DataChannel is still opening.
//! * [`WebRtcManager::handle_data_channel_open`] — flush any buffered
//!   key once the channel reaches the `Open` state.
//! * [`WebRtcManager::prune_expired_ecdh`] — evict stale pending
//!   handshakes so the UI can surface a `handshake_timed_out` flag.

use super::{
  DataChannelState, ECDH_EXCHANGE_TIMEOUT_MS, PeerCrypto, PendingEcdh, WebRtcError, WebRtcManager,
};
use leptos::prelude::Update;
use message::UserId;
use message::error::{ErrorCategory, ErrorCode, ErrorModule};

impl WebRtcManager {
  /// Handle an incoming ECDH public key (received over DataChannel or signaling).
  ///
  /// The `key_data` parameter contains the raw public key bytes (P-256 raw
  /// format, 65 bytes) received directly from the `EcdhKeyExchange` message.
  pub async fn handle_ecdh_key(&self, peer_id: UserId, key_data: &[u8]) -> Result<(), WebRtcError> {
    let public_key = key_data;

    // Check if we already have crypto for this peer (scoped borrow)
    let has_existing = self.inner.borrow().crypto.contains_key(&peer_id);

    if has_existing {
      // Re-keying: remove, update, and re-insert to avoid holding borrow across await
      let mut crypto = self
        .inner
        .borrow_mut()
        .crypto
        .remove(&peer_id)
        .ok_or_else(|| {
          WebRtcError::new(
            ErrorCode::new(ErrorModule::E2e, ErrorCategory::Security, 1),
            "Crypto removed concurrently for peer",
            Some(peer_id.clone()),
          )
        })?;

      crypto
        .import_peer_public_key(public_key)
        .await
        .map_err(|e| {
          WebRtcError::new(
            ErrorCode::new(ErrorModule::E2e, ErrorCategory::Security, 1),
            format!("Failed to import peer public key: {}", e),
            Some(peer_id.clone()),
          )
        })?;

      self
        .inner
        .borrow_mut()
        .crypto
        .insert(peer_id.clone(), crypto);
    } else {
      // First time: create crypto and import peer's public key (all async, no borrow held)
      let mut crypto = PeerCrypto::new(peer_id.clone()).await.map_err(|e| {
        WebRtcError::new(
          ErrorCode::new(ErrorModule::E2e, ErrorCategory::Security, 1),
          format!("Failed to create PeerCrypto: {}", e),
          Some(peer_id.clone()),
        )
      })?;

      crypto
        .import_peer_public_key(public_key)
        .await
        .map_err(|e| {
          WebRtcError::new(
            ErrorCode::new(ErrorModule::E2e, ErrorCategory::Security, 1),
            format!("Failed to import peer public key: {}", e),
            Some(peer_id.clone()),
          )
        })?;

      // Send our public key back
      let our_public_key = crypto.export_public_key().await.map_err(|e| {
        WebRtcError::new(
          ErrorCode::new(ErrorModule::E2e, ErrorCategory::Security, 1),
          format!("Failed to export public key: {}", e),
          Some(peer_id.clone()),
        )
      })?;

      self
        .inner
        .borrow_mut()
        .crypto
        .insert(peer_id.clone(), crypto);

      // Send our ECDH key back via DataChannel (if channel is open)
      self.send_datachannel_ecdh_key_direct(peer_id.clone(), &our_public_key);
    }

    // P1-8 fix: mirror the key-exchange completion into the reactive UI
    // state. The raw key material stays inside `PeerCrypto` as a
    // non-extractable CryptoKey; we only expose the established flag and
    // a logical key_id counter for future rotation support.
    self
      .app_state
      .webrtc_state
      .update(|s| s.mark_encryption_established(&peer_id));

    // Task 19.1 C-1 — flush any control-frame broadcasts that were
    // queued while the handshake was in flight. Must happen after the
    // shared key is installed so `send_encrypted_data_channel_message`
    // succeeds on the drained frames.
    self.flush_pending_broadcast(&peer_id);

    web_sys::console::log_1(
      &format!("[webrtc] Completed ECDH exchange with peer {}", peer_id).into(),
    );

    Ok(())
  }

  /// Send a raw ECDH public key over DataChannel to a peer.
  ///
  /// The `public_key` is in raw P-256 format (65 bytes, uncompressed point).
  ///
  /// P2-11 (Review Round 4): both call sites (`handle_data_channel_open`
  /// and `handle_ecdh_key`) only reach this helper after the DataChannel
  /// has entered the `Open` state, so the "DC not open" branch should
  /// be unreachable in practice. Rather than silently dropping the key,
  /// we now:
  ///
  /// 1. `debug_assert!` in debug builds so regressions surface in tests.
  /// 2. In release builds, re-buffer the key into `pending_ecdh_keys`
  ///    so the eventual `DataChannel.onopen` callback flushes it. This
  ///    keeps the handshake eventually-consistent even if a future
  ///    refactor changes the call order.
  pub(super) fn send_datachannel_ecdh_key_direct(&self, peer_id: UserId, public_key: &[u8]) {
    use message::datachannel::{DataChannelMessage, EcdhKeyExchange};
    use web_sys::RtcDataChannelState;

    let inner = self.inner.borrow();
    let Some(pc) = inner.connections.get(&peer_id) else {
      // No connection at all — nothing we can do here; callers log the
      // missing-peer case when they first look the peer up.
      return;
    };

    let Some(dc) = pc.get_data_channel() else {
      // DataChannel not yet created. Drop the borrow before mutating
      // `pending_ecdh_keys` to avoid a nested borrow.
      drop(inner);
      debug_assert!(
        false,
        "send_datachannel_ecdh_key_direct invoked before DataChannel exists for peer {}",
        peer_id
      );
      self.buffer_pending_ecdh_key(peer_id, public_key.to_vec());
      return;
    };

    if dc.ready_state() != RtcDataChannelState::Open {
      // DataChannel exists but not yet Open. Same safety net as above.
      drop(inner);
      debug_assert!(
        false,
        "send_datachannel_ecdh_key_direct invoked while DataChannel is not Open for peer {}",
        peer_id
      );
      self.buffer_pending_ecdh_key(peer_id, public_key.to_vec());
      return;
    }

    let msg = DataChannelMessage::EcdhKeyExchange(EcdhKeyExchange {
      public_key: public_key.to_vec(),
      timestamp_nanos: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64,
    });
    if let Err(e) = dc.send_message(&msg) {
      web_sys::console::warn_1(&format!("[webrtc] Failed to send ECDH key: {}", e).into());
    } else {
      web_sys::console::log_1(&format!("[webrtc] Sent ECDH key to peer {}", peer_id).into());
    }
  }

  /// Buffer an ECDH public key into `pending_ecdh_keys` so
  /// [`WebRtcManager::handle_data_channel_open`] flushes it once the
  /// DataChannel transitions to the `Open` state. Used as the release-
  /// build fallback in [`WebRtcManager::send_datachannel_ecdh_key_direct`]
  /// when the DataChannel is unexpectedly not open.
  pub(super) fn buffer_pending_ecdh_key(&self, peer_id: UserId, public_key: Vec<u8>) {
    let mut inner = self.inner.borrow_mut();
    inner.pending_ecdh_keys.insert(
      peer_id,
      PendingEcdh {
        public_key,
        started_at_ms: js_sys::Date::now(),
      },
    );
  }

  /// Handle DataChannel open event for a peer.
  ///
  /// Sends any pending ECDH key and updates peer state.
  pub(super) fn handle_data_channel_open(&self, peer_id: UserId) {
    web_sys::console::log_1(&format!("[webrtc] DataChannel opened for peer {}", peer_id).into());

    // Send pending ECDH key if available
    let key_data = {
      let mut inner = self.inner.borrow_mut();
      inner
        .pending_ecdh_keys
        .remove(&peer_id)
        .map(|p| p.public_key)
    };

    if let Some(key_data) = key_data {
      self.send_datachannel_ecdh_key_direct(peer_id.clone(), &key_data);
    }

    // Always clear any prior timeout flag when the DataChannel opens,
    // regardless of whether a pending key was found. A stale
    // `handshake_timed_out` flag from a previous failed attempt should
    // not persist now that the channel is open (P2-21).
    self
      .app_state
      .webrtc_state
      .update(|state| state.clear_encryption_timeout(&peer_id));

    // Update app state: mark data channel as open
    self
      .app_state
      .webrtc_state
      .update(|state| state.update_data_channel_state(&peer_id, DataChannelState::Open));
  }

  /// Sweep the `pending_ecdh_keys` map for entries older than
  /// [`ECDH_EXCHANGE_TIMEOUT_MS`] and remove them (P2-2).
  ///
  /// Runs periodically on a timer (see
  /// [`super::provide_webrtc_manager`]). Two things happen per evicted
  /// peer:
  ///
  /// 1. The entry is dropped from `pending_ecdh_keys` so we do not
  ///    fire once per handshake attempt.
  /// 2. Calls `mark_encryption_timed_out` on `app_state.webrtc_state`
  ///    *outside* the RefCell borrow, letting UI layers surface a
  ///    "key exchange failed" indicator.
  ///
  /// Returns the list of peer IDs that were pruned, so callers can log
  /// or trigger any follow-up actions (e.g. retry via signaling).
  pub fn prune_expired_ecdh(&self) -> Vec<UserId> {
    let now = js_sys::Date::now();
    let expired: Vec<UserId> = {
      let mut inner = self.inner.borrow_mut();
      let to_remove: Vec<UserId> = inner
        .pending_ecdh_keys
        .iter()
        .filter(|(_, pending)| now - pending.started_at_ms >= ECDH_EXCHANGE_TIMEOUT_MS)
        .map(|(id, _)| id.clone())
        .collect();
      for peer_id in &to_remove {
        inner.pending_ecdh_keys.remove(peer_id);
        // Task 19.1 C-1 — handshake timed out; drop any queued
        // control frames so we do not leak them for an ECDH that
        // will never complete.
        inner.pending_broadcast.remove(peer_id);
      }
      to_remove
    };

    // Update reactive UI state outside the RefCell borrow so signal
    // subscribers cannot observe a partially-mutated InnerManager.
    for peer_id in &expired {
      self
        .app_state
        .webrtc_state
        .update(|s| s.mark_encryption_timed_out(peer_id));
      web_sys::console::warn_1(
        &format!(
          "[webrtc] ECDH handshake with peer {} timed out after {}ms",
          peer_id, ECDH_EXCHANGE_TIMEOUT_MS as u64
        )
        .into(),
      );
    }

    expired
  }
}
