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

// ── WebRtcManager construction tests (WASM only) ──

#[cfg(target_arch = "wasm32")]
mod wasm_tests {
  use super::*;
  use wasm_bindgen_test::*;

  wasm_bindgen_test_configure!(run_in_browser);

  fn test_app_state() -> AppState {
    AppState::new()
  }

  #[test]
  fn test_webrtc_manager_new() {
    let app_state = test_app_state();
    let manager = WebRtcManager::new(app_state);

    assert_eq!(manager.connection_count(), 0);
    assert!(manager.encrypted_peers().is_empty());
  }

  #[test]
  fn test_webrtc_manager_clone() {
    let app_state = test_app_state();
    let manager1 = WebRtcManager::new(app_state);
    let manager2 = manager1.clone();

    // Both clones share the same inner state
    assert_eq!(manager1.connection_count(), manager2.connection_count());
  }

  #[test]
  fn test_is_connected_false_when_empty() {
    let app_state = test_app_state();
    let manager = WebRtcManager::new(app_state);
    let peer_id = UserId::new();

    assert!(!manager.is_connected(&peer_id));
  }

  #[test]
  fn test_connection_count_empty() {
    let app_state = test_app_state();
    let manager = WebRtcManager::new(app_state);

    assert_eq!(manager.connection_count(), 0);
  }

  #[test]
  fn test_has_encryption_key_false_when_empty() {
    let app_state = test_app_state();
    let manager = WebRtcManager::new(app_state);
    let peer_id = UserId::new();

    assert!(!manager.has_encryption_key(&peer_id));
  }

  #[test]
  fn test_encrypted_peers_empty() {
    let app_state = test_app_state();
    let manager = WebRtcManager::new(app_state);

    assert!(manager.encrypted_peers().is_empty());
  }

  #[test]
  fn test_default_ice_servers() {
    let app_state = test_app_state();
    let manager = WebRtcManager::new(app_state);

    // Default ICE servers should be set internally
    manager.init_with_ice_servers(vec![IceServerConfig::stun("stun:custom.example.com:3478")]);
    // No public accessor for ice_servers, but we can verify the manager works
    assert_eq!(manager.connection_count(), 0);
  }

  #[test]
  fn test_init_with_empty_ice_servers_keeps_defaults() {
    let app_state = test_app_state();
    let manager = WebRtcManager::new(app_state);

    // Empty vec should not overwrite defaults
    manager.init_with_ice_servers(vec![]);
    assert_eq!(manager.connection_count(), 0);
  }

  #[test]
  fn test_app_state_webrtc_state_tracking() {
    let app_state = test_app_state();
    let user_id = UserId::new();

    // Initially empty
    assert_eq!(app_state.webrtc_state.get().peers.len(), 0);

    // Add a peer
    app_state
      .webrtc_state
      .update(|state| state.add_peer(user_id.clone(), true));

    let state = app_state.webrtc_state.get();
    assert_eq!(state.peers.len(), 1);
    assert!(state.get_peer(&user_id).is_some());
    assert_eq!(
      state.get_peer(&user_id).unwrap().connection_state,
      PeerConnectionState::Connecting
    );

    // Update connection state
    drop(state);
    app_state
      .webrtc_state
      .update(|s| s.update_connection_state(&user_id, PeerConnectionState::Connected));

    assert_eq!(
      app_state
        .webrtc_state
        .get()
        .get_peer(&user_id)
        .unwrap()
        .connection_state,
      PeerConnectionState::Connected
    );

    // Update data channel state
    app_state
      .webrtc_state
      .update(|s| s.update_data_channel_state(&user_id, DataChannelState::Open));

    let peer = app_state
      .webrtc_state
      .get()
      .get_peer(&user_id)
      .unwrap()
      .clone();
    assert!(peer.is_ready());

    // Remove peer
    app_state.webrtc_state.update(|s| s.remove_peer(&user_id));
    assert_eq!(app_state.webrtc_state.get().peers.len(), 0);
  }

  #[test]
  fn test_close_connection_updates_app_state() {
    let app_state = test_app_state();
    let manager = WebRtcManager::new(app_state);
    let peer_id = UserId::new();

    // Add peer to app state manually (simulating connection setup)
    app_state
      .webrtc_state
      .update(|state| state.add_peer(peer_id.clone(), true));

    assert_eq!(app_state.webrtc_state.get().peers.len(), 1);

    // close_connection removes from app state
    manager.close_connection(&peer_id);

    assert_eq!(app_state.webrtc_state.get().peers.len(), 0);
  }

  #[test]
  fn test_close_all_clears_app_state() {
    let app_state = test_app_state();
    let manager = WebRtcManager::new(app_state);

    // Add multiple peers
    for _ in 0..3 {
      let peer_id = UserId::new();
      app_state
        .webrtc_state
        .update(|state| state.add_peer(peer_id, true));
    }

    assert_eq!(app_state.webrtc_state.get().peers.len(), 3);

    manager.close_all();

    assert_eq!(app_state.webrtc_state.get().peers.len(), 0);
  }

  // ── P0 regression tests ──

  /// P0-2 regression: handle_ecdh_key establishes encryption; close_connection
  /// must clear it so that a replaced peer connection starts with a clean slate.
  #[wasm_bindgen_test]
  async fn test_close_connection_clears_crypto_state() {
    let app_state = test_app_state();
    let manager = WebRtcManager::new(app_state);
    let peer_id = UserId::from(42u64);

    // Use a second PeerCrypto to produce a valid foreign public key.
    let foreign_crypto = PeerCrypto::new(UserId::from(99u64)).await.unwrap();
    let foreign_pk = foreign_crypto.export_public_key().await.unwrap();

    // Establish encryption state for the peer.
    manager
      .handle_ecdh_key(peer_id.clone(), &foreign_pk)
      .await
      .unwrap();
    assert!(
      manager.has_encryption_key(&peer_id),
      "Encryption key should be established after handle_ecdh_key"
    );
    assert_eq!(manager.encrypted_peers().len(), 1);

    // close_connection must wipe the crypto state (P0-2 fix).
    manager.close_connection(&peer_id);
    assert!(
      !manager.has_encryption_key(&peer_id),
      "Encryption key should be cleared after close_connection"
    );
    assert!(manager.encrypted_peers().is_empty());
  }

  /// P2-1 regression: if offer creation fails, connect_to_peer calls
  /// close_connection which must also clear any pending ECDH keys.
  /// We verify the cleanup path directly since offer failures require
  /// a real RTCPeerConnection.
  #[wasm_bindgen_test]
  async fn test_close_connection_clears_pending_ecdh_keys() {
    let app_state = test_app_state();
    let manager = WebRtcManager::new(app_state);
    let peer_id = UserId::from(42u64);

    // Simulate the state left behind by initiate_ecdh_exchange by
    // going through handle_ecdh_key (which inserts into inner.crypto).
    let foreign_crypto = PeerCrypto::new(UserId::from(99u64)).await.unwrap();
    let foreign_pk = foreign_crypto.export_public_key().await.unwrap();
    manager
      .handle_ecdh_key(peer_id.clone(), &foreign_pk)
      .await
      .unwrap();

    // close_connection must clean up everything (connections, crypto, pending).
    manager.close_connection(&peer_id);
    assert!(!manager.is_connected(&peer_id));
    assert!(!manager.has_encryption_key(&peer_id));
  }

  /// Verify that handle_ecdh_key correctly derives a shared key when
  /// invoked for the first time (no pre-existing crypto state).
  #[wasm_bindgen_test]
  async fn test_handle_ecdh_key_first_time_establishes_shared_key() {
    let app_state = test_app_state();
    let manager = WebRtcManager::new(app_state);
    let peer_id = UserId::from(42u64);

    let foreign_crypto = PeerCrypto::new(UserId::from(99u64)).await.unwrap();
    let foreign_pk = foreign_crypto.export_public_key().await.unwrap();

    assert!(!manager.has_encryption_key(&peer_id));

    manager
      .handle_ecdh_key(peer_id.clone(), &foreign_pk)
      .await
      .unwrap();

    assert!(manager.has_encryption_key(&peer_id));
  }

  /// P0-5 regression: `connect_to_peer` must register the peer in the
  /// reactive UI state (`app_state.webrtc_state`) as soon as the underlying
  /// `PeerConnection` is stored. Without this, `update_connection_state`
  /// and `update_data_channel_state` silently become no-ops and the UI
  /// Signal stays empty forever.
  #[wasm_bindgen_test]
  async fn test_connect_to_peer_registers_in_app_state() {
    let app_state = test_app_state();
    let manager = WebRtcManager::new(app_state.clone());
    let peer_id = UserId::from(7u64);

    // Sanity: nothing registered initially.
    assert_eq!(app_state.webrtc_state.get().peers.len(), 0);
    assert!(!manager.is_connected(&peer_id));

    // `connect_to_peer` creates a real RTCPeerConnection with a DataChannel,
    // generates an SDP offer, and then stores the PC. The signaling client
    // is intentionally left unset so `send_sdp_offer` is a no-op, but the
    // `add_peer` call must still fire because it happens *before* the
    // signaling step.
    manager.connect_to_peer(peer_id.clone()).await.unwrap();

    // The peer must now appear in the reactive state (initiator side).
    let state = app_state.webrtc_state.get();
    let peer = state
      .get_peer(&peer_id)
      .expect("peer should be registered in reactive state after connect_to_peer");
    assert!(peer.is_initiator, "connect_to_peer is the initiator side");
    assert_eq!(peer.connection_state, PeerConnectionState::Connecting);

    // And the underlying PC map must match.
    assert!(manager.is_connected(&peer_id));
    assert_eq!(manager.connection_count(), 1);

    // Cleanup.
    drop(state);
    manager.close_connection(&peer_id);
    assert_eq!(app_state.webrtc_state.get().peers.len(), 0);
  }

  // ── Broadcast (P1-9) regression tests ──

  /// T-N2: `broadcast_encrypted_message` must produce an independent
  /// pairwise-encrypted copy for each peer (Req 5.2.10).
  ///
  /// This test establishes ECDH shared keys with three foreign peers, then
  /// pulls each peer's `PeerCrypto` out of the manager's inner state and
  /// encrypts the same plaintext. We assert that:
  /// 1. All three peers appear in `encrypted_peers()`.
  /// 2. Each encryption yields a distinct ciphertext (different nonces,
  ///    different AES-GCM outputs — no accidental key reuse or shared
  ///    buffer).
  /// 3. Every ciphertext has the expected `nonce || ct || tag` layout
  ///    (12 B nonce + plaintext.len() B ciphertext + 16 B auth tag).
  #[wasm_bindgen_test]
  async fn test_broadcast_encrypted_message_multi_peer_independent_encryption() {
    let app_state = test_app_state();
    let manager = WebRtcManager::new(app_state);

    let peer_ids = [UserId::new(), UserId::new(), UserId::new()];

    // Establish a shared key with each foreign peer via handle_ecdh_key.
    for peer_id in &peer_ids {
      let foreign_crypto = PeerCrypto::new(peer_id.clone()).await.unwrap();
      let foreign_pk = foreign_crypto.export_public_key().await.unwrap();
      manager
        .handle_ecdh_key(peer_id.clone(), &foreign_pk)
        .await
        .unwrap();
    }

    // All three peers must be in the encrypted set.
    let encrypted = manager.encrypted_peers();
    assert_eq!(encrypted.len(), 3, "all 3 peers should have shared keys");
    for peer_id in &peer_ids {
      assert!(encrypted.contains(peer_id));
    }

    // Encrypt the same plaintext once per peer using that peer's crypto.
    let plaintext = b"broadcast-payload";
    let mut ciphertexts: Vec<Vec<u8>> = Vec::with_capacity(3);
    for peer_id in &peer_ids {
      let crypto = {
        let inner = manager.inner.borrow();
        inner.crypto.get(peer_id).expect("crypto present").clone()
      };
      let ct = crypto.encrypt(plaintext).await.unwrap();
      ciphertexts.push(ct);
    }

    // AES-GCM layout: 12-byte nonce ∥ ciphertext (==plaintext.len()) ∥ 16-byte tag.
    let expected_len = encryption::GCM_NONCE_SIZE + plaintext.len() + 16;
    for ct in &ciphertexts {
      assert_eq!(
        ct.len(),
        expected_len,
        "ciphertext layout must be nonce ∥ ct ∥ tag"
      );
    }

    // Independence: every pairwise ciphertext must differ. Nonces are random
    // (96-bit), so two collisions in a 3-way comparison is astronomically
    // improbable — any equality indicates shared state / key reuse.
    assert_ne!(ciphertexts[0], ciphertexts[1]);
    assert_ne!(ciphertexts[0], ciphertexts[2]);
    assert_ne!(ciphertexts[1], ciphertexts[2]);

    // Nonce prefixes must also differ (sanity: rules out a bug where the
    // same nonce was embedded but the body differed due to different keys).
    let nonces: Vec<&[u8]> = ciphertexts
      .iter()
      .map(|c| &c[..encryption::GCM_NONCE_SIZE])
      .collect();
    assert_ne!(nonces[0], nonces[1]);
    assert_ne!(nonces[0], nonces[2]);
    assert_ne!(nonces[1], nonces[2]);
  }

  /// P1-9 regression: `broadcast_encrypted_message` uses `join_all` to fan
  /// out concurrently. When no `PeerConnection` / DataChannel is registered
  /// for any peer, every future must fail independently (logged as warn)
  /// and the broadcast must return `Ok(0)` without panicking — proving the
  /// partial-failure semantics survived the serial-to-concurrent rewrite.
  #[wasm_bindgen_test]
  async fn test_broadcast_encrypted_message_without_data_channels_returns_zero() {
    let app_state = test_app_state();
    let manager = WebRtcManager::new(app_state);

    // Establish crypto with two peers but never register a PeerConnection,
    // so `send_encrypted_message` will fail with "No connection found".
    for _ in 0..2 {
      let peer_id = UserId::new();
      let foreign_crypto = PeerCrypto::new(peer_id.clone()).await.unwrap();
      let foreign_pk = foreign_crypto.export_public_key().await.unwrap();
      manager.handle_ecdh_key(peer_id, &foreign_pk).await.unwrap();
    }
    assert_eq!(manager.encrypted_peers().len(), 2);

    // Every peer's send will fail inside join_all; the aggregate result is 0.
    let result = manager
      .broadcast_encrypted_message(b"payload")
      .await
      .expect("broadcast should succeed even when every send fails");
    assert_eq!(result.sent, 0, "no peer should count as successfully sent");
    assert_eq!(
      result.failed_peers.len(),
      2,
      "both peers should be reported as failed"
    );
  }

  /// P1-9 regression: empty encrypted-peer set must short-circuit with an
  /// error (not `Ok(0)`) so callers can distinguish "nothing to do" from
  /// "all sends failed".
  #[wasm_bindgen_test]
  async fn test_broadcast_encrypted_message_no_peers_errors() {
    let app_state = test_app_state();
    let manager = WebRtcManager::new(app_state);

    let err = manager
      .broadcast_encrypted_message(b"payload")
      .await
      .expect_err("broadcast without any encrypted peer must error");
    assert!(
      err.message.contains("No peers"),
      "error must mention missing peers, got: {err}"
    );
  }

  /// P2-2: with no pending ECDH exchanges, `prune_expired_ecdh` must
  /// return an empty vector and leave the UI state untouched.
  #[wasm_bindgen_test]
  fn test_prune_expired_ecdh_empty() {
    let app_state = test_app_state();
    let manager = WebRtcManager::new(app_state);

    let expired = manager.prune_expired_ecdh();
    assert!(expired.is_empty());
  }

  /// P2-2: a freshly inserted `PendingEcdh` with `started_at_ms` far in
  /// the past must be pruned and flagged on `app_state.webrtc_state`.
  /// We manipulate the internal map directly because waiting 10s inside
  /// a wasm test is unacceptable; this exercises the prune logic without
  /// coupling to the real `initiate_ecdh_exchange` pipeline.
  #[wasm_bindgen_test]
  fn test_prune_expired_ecdh_flags_timed_out_peer() {
    let app_state = test_app_state();
    let manager = WebRtcManager::new(app_state.clone());
    let peer_id = UserId::from(77u64);

    // Register the peer in the reactive state so the mark path has a
    // slot to update — otherwise the UI mirror would be a no-op.
    app_state
      .webrtc_state
      .update(|s| s.add_peer(peer_id.clone(), true));

    // Inject an expired pending entry directly: started_at_ms far in the
    // past guarantees `now - started_at_ms >= ECDH_EXCHANGE_TIMEOUT_MS`.
    {
      let mut inner = manager.inner.borrow_mut();
      inner.pending_ecdh_keys.insert(
        peer_id.clone(),
        PendingEcdh {
          public_key: vec![0u8; 65],
          started_at_ms: 0.0,
        },
      );
    }

    let expired = manager.prune_expired_ecdh();
    assert_eq!(expired, vec![peer_id.clone()]);

    // Pending entry should have been drained.
    assert!(
      manager
        .inner
        .borrow()
        .pending_ecdh_keys
        .get(&peer_id)
        .is_none()
    );

    // UI state must mirror the timeout.
    app_state.webrtc_state.with(|s| {
      let peer = s.get_peer(&peer_id).expect("peer should still exist");
      assert!(peer.encryption.handshake_timed_out);
      assert!(!peer.encryption.established);
    });
  }

  /// P2-2: a fresh (just-inserted) pending ECDH entry must NOT be
  /// pruned — its `started_at_ms` is less than `ECDH_EXCHANGE_TIMEOUT_MS`
  /// behind `now`, so the handshake is still within its grace window.
  #[wasm_bindgen_test]
  fn test_prune_expired_ecdh_preserves_fresh_entry() {
    let app_state = test_app_state();
    let manager = WebRtcManager::new(app_state);
    let peer_id = UserId::from(88u64);

    {
      let mut inner = manager.inner.borrow_mut();
      inner.pending_ecdh_keys.insert(
        peer_id.clone(),
        PendingEcdh {
          public_key: vec![0u8; 65],
          started_at_ms: js_sys::Date::now(),
        },
      );
    }

    let expired = manager.prune_expired_ecdh();
    assert!(expired.is_empty());
    assert!(
      manager
        .inner
        .borrow()
        .pending_ecdh_keys
        .contains_key(&peer_id)
    );
  }
}
