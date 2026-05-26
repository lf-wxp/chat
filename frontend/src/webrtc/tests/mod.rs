//! WebRTC manager unit tests.
//!
//! Tests the core WebRtcManager logic including peer state tracking,
//! encryption key management, and connection lifecycle.
//!
//! Note: Tests that require `AppState` are WASM-only because `AppState::new()`
//! accesses browser APIs. The `#[cfg(target_arch = "wasm32")]` gate is used
//! for those tests.
//!
//! # Test organization
//! - `types/tests.rs` covers `PeerState`, `WebRtcState`, `PeerConnectionState`,
//!   and `DataChannelState` in detail.
//! - This file focuses on module-level integration and `WebRtcManager` behaviour.

use super::*;
use message::datachannel::{ChatText, DataChannelMessage};

// ── DataChannel message discriminator tests ──

#[test]
fn test_datachannel_message_discriminators() {
  let chat = DataChannelMessage::ChatText(ChatText {
    message_id: message::MessageId(uuid::Uuid::new_v4()),
    content: "test".to_string(),
    reply_to: None,
    timestamp_nanos: 0,
    room_id: None,
    mentions: vec![],
  });
  assert_eq!(chat.discriminator(), 0x80);

  let ecdh = DataChannelMessage::EcdhKeyExchange(message::datachannel::EcdhKeyExchange {
    public_key: vec![0u8; 65], // P-256 raw format: 65 bytes
    timestamp_nanos: 0,
  });
  assert_eq!(ecdh.discriminator(), 0xA0);
}

// ── Mesh topology limit tests ──

#[test]
fn test_max_mesh_peers_constant() {
  // Per requirements: maximum 8 peers in mesh
  assert_eq!(MAX_MESH_PEERS, 8);
}

// ── Encryption constants tests ──

#[test]
fn test_gcm_nonce_size() {
  // AES-GCM nonce must be 12 bytes per NIST recommendation
  assert_eq!(encryption::GCM_NONCE_SIZE, 12);
}

#[test]
fn test_aes_key_size() {
  // AES-256 uses 256-bit (32-byte) keys
  assert_eq!(encryption::AES_KEY_SIZE, 256);
}

// ── Task 19.1 envelope protocol tests ──

/// The envelope marker byte must not collide with any value returned
/// by `DataChannelMessage::discriminator()`. Otherwise the receive
/// path could mis-route a plaintext frame as an encrypted envelope
/// (or vice versa), opening a downgrade / parsing hazard.
#[test]
fn encrypted_marker_disjoint_from_every_discriminator() {
  use crate::webrtc::data_channel::ENCRYPTED_MARKER;

  let samples = [
    DataChannelMessage::ChatText(ChatText {
      message_id: message::MessageId(uuid::Uuid::new_v4()),
      content: "m".into(),
      reply_to: None,
      timestamp_nanos: 0,
      room_id: None,
      mentions: vec![],
    })
    .discriminator(),
    DataChannelMessage::EcdhKeyExchange(message::datachannel::EcdhKeyExchange {
      public_key: vec![0u8; 65],
      timestamp_nanos: 0,
    })
    .discriminator(),
  ];
  for d in samples {
    assert_ne!(
      d, ENCRYPTED_MARKER,
      "discriminator 0x{d:02X} collides with ENCRYPTED_MARKER"
    );
  }
  // 0xFE sits above the 0xC3 ceiling used by current message kinds.
  const { assert!(crate::webrtc::data_channel::ENCRYPTED_MARKER > 0xC3) };
}

#[cfg(target_arch = "wasm32")]
mod wasm_broadcast;
#[cfg(target_arch = "wasm32")]
mod wasm_crypto;
#[cfg(target_arch = "wasm32")]
mod wasm_manager;
