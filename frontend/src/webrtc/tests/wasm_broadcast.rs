use super::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn test_app_state() -> AppState {
  AppState::new()
}

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
