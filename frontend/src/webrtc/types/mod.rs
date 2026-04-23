//! WebRTC connection types.
//!
//! Defines types for managing RTCPeerConnection state, DataChannel state,
//! and encryption status.

use message::UserId;
use std::collections::HashMap;

/// Connection state for a peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerConnectionState {
  /// Connection is being established.
  Connecting,
  /// Connection is established (ICE + DTLS complete).
  Connected,
  /// Connection is disconnected (may recover).
  Disconnected,
  /// Connection has failed (will not recover).
  Failed,
  /// Connection is closed.
  Closed,
}

impl From<&str> for PeerConnectionState {
  fn from(state: &str) -> Self {
    match state {
      "connecting" => Self::Connecting,
      "connected" => Self::Connected,
      "disconnected" => Self::Disconnected,
      "failed" => Self::Failed,
      "closed" => Self::Closed,
      _ => Self::Closed,
    }
  }
}

/// DataChannel state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataChannelState {
  /// DataChannel is being established.
  Connecting,
  /// DataChannel is open and ready for data transfer.
  Open,
  /// DataChannel is closing.
  Closing,
  /// DataChannel is closed.
  Closed,
}

impl From<&str> for DataChannelState {
  fn from(state: &str) -> Self {
    match state {
      "connecting" => Self::Connecting,
      "open" => Self::Open,
      "closing" => Self::Closing,
      "closed" => Self::Closed,
      _ => Self::Closed,
    }
  }
}

/// Tracks the E2EE key-exchange status of a peer, without storing the
/// raw key material.
///
/// The actual AES-256-GCM key is a non-extractable `CryptoKey` owned by
/// `PeerCrypto` in `webrtc::encryption`. Exposing raw key bytes would
/// defeat the purpose of using the Web Crypto API, so this type only
/// mirrors whether a shared key has been established and tracks the
/// logical key-id for future key-rotation support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PeerEncryptionStatus {
  /// Logical key-id for rotation support. Starts at 0 and is incremented
  /// every time a fresh ECDH exchange is performed for the peer.
  pub key_id: u32,
  /// True once an ECDH shared key has been derived and imported as an
  /// AES-256-GCM key for this peer.
  pub established: bool,
  /// True when the most recent ECDH handshake attempt timed out before
  /// the peer responded with its public key (P2-2). UI layers observe
  /// this flag to surface a "key exchange failed" indicator. Cleared
  /// automatically by `mark_encryption_established` (successful handshake
  /// supersedes a prior timeout) and by `clear_encryption`.
  pub handshake_timed_out: bool,
}

impl PeerEncryptionStatus {
  // Intentionally no `new()` — use `PeerEncryptionStatus::default()` or
  // explicit field construction to avoid diverging init paths (P2-13 fix).
}

/// Tracks all WebRTC state for a single peer.
#[derive(Debug, Clone)]
pub struct PeerState {
  /// Remote user ID.
  pub user_id: UserId,
  /// Current connection state.
  pub connection_state: PeerConnectionState,
  /// DataChannel state (if established).
  pub data_channel_state: Option<DataChannelState>,
  /// E2EE key-exchange status (mirrors `PeerCrypto` readiness).
  pub encryption: PeerEncryptionStatus,
  /// Whether we are the initiator (offer sender).
  pub is_initiator: bool,
}

impl PeerState {
  /// Create a new peer state.
  pub fn new(user_id: UserId, is_initiator: bool) -> Self {
    Self {
      user_id,
      connection_state: PeerConnectionState::Connecting,
      data_channel_state: None,
      encryption: PeerEncryptionStatus::default(),
      is_initiator,
    }
  }

  /// Check if the peer is fully connected (connection + data channel open).
  pub fn is_ready(&self) -> bool {
    self.connection_state == PeerConnectionState::Connected
      && self.data_channel_state == Some(DataChannelState::Open)
  }
}

/// Global WebRTC state managed by the application.
#[derive(Debug, Clone)]
pub struct WebRtcState {
  /// All active peer connections, keyed by user ID.
  pub peers: HashMap<UserId, PeerState>,
}

impl WebRtcState {
  /// Create a new empty WebRTC state.
  pub fn new() -> Self {
    Self {
      peers: HashMap::new(),
    }
  }

  /// Add a new peer.
  pub fn add_peer(&mut self, user_id: UserId, is_initiator: bool) {
    let state = PeerState::new(user_id.clone(), is_initiator);
    self.peers.insert(user_id, state);
  }

  /// Remove a peer.
  pub fn remove_peer(&mut self, user_id: &UserId) {
    self.peers.remove(user_id);
  }

  /// Get a peer's state.
  pub fn get_peer(&self, user_id: &UserId) -> Option<&PeerState> {
    self.peers.get(user_id)
  }

  /// Get a mutable reference to a peer's state.
  pub fn get_peer_mut(&mut self, user_id: &UserId) -> Option<&mut PeerState> {
    self.peers.get_mut(user_id)
  }

  /// Update a peer's connection state.
  pub fn update_connection_state(&mut self, user_id: &UserId, state: PeerConnectionState) {
    if let Some(peer) = self.peers.get_mut(user_id) {
      peer.connection_state = state;
    }
  }

  /// Update a peer's data channel state.
  pub fn update_data_channel_state(&mut self, user_id: &UserId, state: DataChannelState) {
    if let Some(peer) = self.peers.get_mut(user_id) {
      peer.data_channel_state = Some(state);
    }
  }

  /// Mark the E2EE key exchange as established for a peer and bump the
  /// logical `key_id`. Called after `PeerCrypto` has successfully derived
  /// and imported the AES-256-GCM shared key.
  ///
  /// Also clears any prior `handshake_timed_out` flag — a successful
  /// handshake supersedes an earlier timeout observation (P2-2).
  pub fn mark_encryption_established(&mut self, user_id: &UserId) {
    if let Some(peer) = self.peers.get_mut(user_id) {
      peer.encryption.key_id = peer.encryption.key_id.wrapping_add(1);
      peer.encryption.established = true;
      peer.encryption.handshake_timed_out = false;
    }
  }

  /// Mark that the most recent ECDH handshake attempt with the peer has
  /// timed out (P2-2). Leaves `key_id` unchanged so a subsequent
  /// successful handshake increments monotonically.
  pub fn mark_encryption_timed_out(&mut self, user_id: &UserId) {
    if let Some(peer) = self.peers.get_mut(user_id) {
      peer.encryption.established = false;
      peer.encryption.handshake_timed_out = true;
    }
  }

  /// Reset the E2EE status for a peer (used on disconnect / key rotation).
  pub fn clear_encryption(&mut self, user_id: &UserId) {
    if let Some(peer) = self.peers.get_mut(user_id) {
      peer.encryption.established = false;
      peer.encryption.handshake_timed_out = false;
    }
  }

  /// Clear only the `handshake_timed_out` flag for a peer.
  ///
  /// Called when a DataChannel opens and a pending ECDH key is successfully
  /// sent, so the UI no longer shows a timeout even if the handshake
  /// completed after the timer expired (P2-18).
  pub fn clear_encryption_timeout(&mut self, user_id: &UserId) {
    if let Some(peer) = self.peers.get_mut(user_id) {
      peer.encryption.handshake_timed_out = false;
    }
  }

  /// Get the number of connected peers.
  pub fn connected_count(&self) -> usize {
    self.peers.values().filter(|p| p.is_ready()).count()
  }
}

impl Default for WebRtcState {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests;
