use super::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// `PeerCrypto::new()` should generate an ECDH P-256 key pair and
/// `has_shared_key()` should return false until a peer key is imported.
#[wasm_bindgen_test]
async fn test_ecdh_key_generation() {
  let crypto = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  assert!(
    !crypto.has_shared_key(),
    "Shared key should not be established yet"
  );
  assert_eq!(crypto.key_id(), 0);
}

/// The exported public key must be a 65-byte raw uncompressed EC point.
#[wasm_bindgen_test]
async fn test_export_public_key_length() {
  let crypto = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let pk = crypto.export_public_key().await.unwrap();
  assert_eq!(
    pk.len(),
    65,
    "P-256 raw uncompressed public key should be 65 bytes"
  );
  assert_eq!(
    pk[0], 0x04,
    "First byte should be 0x04 (uncompressed point marker)"
  );
}

/// Two `PeerCrypto` instances should successfully derive a shared key
/// after exchanging public keys.
#[wasm_bindgen_test]
async fn test_shared_key_establishment() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let mut bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();

  let alice_pk = alice.export_public_key().await.unwrap();
  let bob_pk = bob.export_public_key().await.unwrap();

  alice.import_peer_public_key(&bob_pk).await.unwrap();
  bob.import_peer_public_key(&alice_pk).await.unwrap();

  assert!(
    alice.has_shared_key(),
    "Alice should have shared key after import"
  );
  assert!(
    bob.has_shared_key(),
    "Bob should have shared key after import"
  );
  assert_eq!(alice.key_id(), 1);
  assert_eq!(bob.key_id(), 1);
}

/// AES-256-GCM encrypt/decrypt round-trip: decrypting the ciphertext
/// must recover the original plaintext.
#[wasm_bindgen_test]
async fn test_encrypt_decrypt_roundtrip() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let mut bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();

  let alice_pk = alice.export_public_key().await.unwrap();
  let bob_pk = bob.export_public_key().await.unwrap();

  alice.import_peer_public_key(&bob_pk).await.unwrap();
  bob.import_peer_public_key(&alice_pk).await.unwrap();

  let plaintext = b"Hello, secure WebRTC world!";
  let ciphertext = alice.encrypt(plaintext).await.unwrap();

  // Ciphertext must contain nonce + encrypted data
  assert!(ciphertext.len() > GCM_NONCE_SIZE);

  let decrypted = bob.decrypt(&ciphertext).await.unwrap();
  assert_eq!(decrypted, plaintext.as_slice());
}

/// Decrypting without an established shared key must return an error.
#[wasm_bindgen_test]
async fn test_decrypt_without_shared_key_fails() {
  let crypto = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let dummy = vec![0u8; GCM_NONCE_SIZE + 16];
  let result = crypto.decrypt(&dummy).await;
  assert!(result.is_err(), "Decryption should fail without shared key");
}

/// Tampering with a single ciphertext byte must cause decryption to
/// fail because AES-GCM verifies the authentication tag.
#[wasm_bindgen_test]
async fn test_decrypt_tampered_ciphertext_fails() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let mut bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();

  let alice_pk = alice.export_public_key().await.unwrap();
  let bob_pk = bob.export_public_key().await.unwrap();

  alice.import_peer_public_key(&bob_pk).await.unwrap();
  bob.import_peer_public_key(&alice_pk).await.unwrap();

  let plaintext = b"Tamper-proof message";
  let mut ciphertext = alice.encrypt(plaintext).await.unwrap();

  // Tamper with the ciphertext (skip nonce, modify first encrypted byte)
  let idx = GCM_NONCE_SIZE + 1;
  ciphertext[idx] = ciphertext[idx].wrapping_add(1);

  let result = bob.decrypt(&ciphertext).await;
  assert!(
    result.is_err(),
    "Decryption should fail on tampered ciphertext"
  );
}

/// After a full key exchange, both `PeerCrypto` instances must derive
/// the same shared key, verified by cross encrypt/decrypt.
#[wasm_bindgen_test]
async fn test_cross_encrypt_decrypt() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let mut bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();

  let alice_pk = alice.export_public_key().await.unwrap();
  let bob_pk = bob.export_public_key().await.unwrap();

  alice.import_peer_public_key(&bob_pk).await.unwrap();
  bob.import_peer_public_key(&alice_pk).await.unwrap();

  // Alice encrypts, Bob decrypts
  let msg1 = b"Alice -> Bob";
  let ct1 = alice.encrypt(msg1).await.unwrap();
  assert_eq!(bob.decrypt(&ct1).await.unwrap(), msg1.as_slice());

  // Bob encrypts, Alice decrypts
  let msg2 = b"Bob -> Alice";
  let ct2 = bob.encrypt(msg2).await.unwrap();
  assert_eq!(alice.decrypt(&ct2).await.unwrap(), msg2.as_slice());
}

/// Ciphertext layout check: the first `GCM_NONCE_SIZE` bytes are the
/// nonce, and each encryption must produce a fresh nonce.
#[wasm_bindgen_test]
async fn test_nonce_uniqueness() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let mut bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();

  alice
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();
  bob
    .import_peer_public_key(&alice.export_public_key().await.unwrap())
    .await
    .unwrap();

  let plaintext = b"test";
  let ct1 = alice.encrypt(plaintext).await.unwrap();
  let ct2 = alice.encrypt(plaintext).await.unwrap();

  let nonce1 = &ct1[..GCM_NONCE_SIZE];
  let nonce2 = &ct2[..GCM_NONCE_SIZE];
  assert_ne!(nonce1, nonce2, "Nonces should be unique across encryptions");
}
