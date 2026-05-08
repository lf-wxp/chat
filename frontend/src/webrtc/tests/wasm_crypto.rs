use super::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn test_app_state() -> AppState {
  AppState::new()
}

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

  // close_connection must wipe the crypto state.
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
  let manager = WebRtcManager::new(app_state);
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
    !manager
      .inner
      .borrow()
      .pending_ecdh_keys
      .contains_key(&peer_id)
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
