//! WebRTC connection types.
//!
//! Defines types for managing RTCPeerConnection state, DataChannel state,
//! and encryption keys.

use message::UserId;
use std::collections::HashMap;

/// Unique identifier for a WebRTC peer connection.
#[allow(dead_code)]
pub type PeerId = UserId;

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

/// Stores the encryption keys for a peer-to-peer connection.
///
/// Uses ECDH P-256 for key exchange and AES-256-GCM for encryption.
#[derive(Debug, Clone)]
pub struct PeerEncryptionKeys {
  /// The AES-256 key derived from ECDH exchange (32 bytes).
  pub aes_key: Vec<u8>,
  /// The key ID for key rotation support.
  pub key_id: u32,
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
  /// Encryption keys (if ECDH exchange completed).
  pub encryption_keys: Option<PeerEncryptionKeys>,
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
      encryption_keys: None,
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

  /// Set encryption keys for a peer.
  pub fn set_encryption_keys(&mut self, user_id: &UserId, keys: PeerEncryptionKeys) {
    if let Some(peer) = self.peers.get_mut(user_id) {
      peer.encryption_keys = Some(keys);
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
