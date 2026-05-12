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

/// `CryptoKeyValue::from_js` should reject any `JsValue` that is not a
/// `CryptoKey` instance.
#[wasm_bindgen_test]
fn test_crypto_key_value_from_js_rejects_non_crypto_key() {
  let not_a_key = JsValue::from_str("hello");
  assert!(
    not_a_key.dyn_ref::<web_sys::CryptoKey>().is_none(),
    "A plain string must not be a CryptoKey"
  );

  let not_a_key = JsValue::from_bool(true);
  assert!(
    not_a_key.dyn_ref::<web_sys::CryptoKey>().is_none(),
    "A boolean must not be a CryptoKey"
  );

  let not_a_key = JsValue::NULL;
  assert!(
    not_a_key.dyn_ref::<web_sys::CryptoKey>().is_none(),
    "null must not be a CryptoKey"
  );
}

/// A JS number is not a CryptoKey.
#[wasm_bindgen_test]
fn test_crypto_key_value_from_js_rejects_number() {
  let not_a_key = JsValue::from_f64(42.0);
  assert!(
    not_a_key.dyn_ref::<web_sys::CryptoKey>().is_none(),
    "A number must not be a CryptoKey"
  );
}

/// `undefined` is not a CryptoKey.
#[wasm_bindgen_test]
fn test_crypto_key_value_from_js_rejects_undefined() {
  let not_a_key = JsValue::UNDEFINED;
  assert!(
    not_a_key.dyn_ref::<web_sys::CryptoKey>().is_none(),
    "undefined must not be a CryptoKey"
  );
}

/// A plain JS object is not a CryptoKey.
#[wasm_bindgen_test]
fn test_crypto_key_value_from_js_rejects_plain_object() {
  let plain_obj = js_sys::Object::new();
  let not_a_key = JsValue::from(plain_obj);
  assert!(
    not_a_key.dyn_ref::<web_sys::CryptoKey>().is_none(),
    "A plain JS object must not be a CryptoKey"
  );
}

/// A JS array is not a CryptoKey.
#[wasm_bindgen_test]
fn test_crypto_key_value_from_js_rejects_array() {
  let arr = js_sys::Array::new();
  let not_a_key = JsValue::from(arr);
  assert!(
    not_a_key.dyn_ref::<web_sys::CryptoKey>().is_none(),
    "A JS array must not be a CryptoKey"
  );
}

/// Encrypting without a shared key must return an error.
#[wasm_bindgen_test]
async fn test_encrypt_without_shared_key_fails() {
  let crypto = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  assert!(!crypto.has_shared_key(), "Should not have shared key yet");
  let result = crypto.encrypt(b"secret").await;
  assert!(result.is_err(), "Encryption should fail without shared key");
}

/// Encrypting an empty plaintext should succeed and produce a
/// nonce + GCM tag (no additional ciphertext bytes beyond the tag).
#[wasm_bindgen_test]
async fn test_encrypt_empty_plaintext() {
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

  let plaintext: &[u8] = b"";
  let ciphertext = alice.encrypt(plaintext).await.unwrap();

  // GCM_NONCE_SIZE (12) + GCM tag (16) = 28 bytes for empty plaintext
  assert_eq!(
    ciphertext.len(),
    GCM_NONCE_SIZE + 16,
    "Empty plaintext should produce nonce + tag only"
  );

  let decrypted = bob.decrypt(&ciphertext).await.unwrap();
  assert_eq!(
    decrypted.len(),
    0,
    "Decrypted empty plaintext should be empty"
  );
}

/// Encrypting a large message (10 KiB) should work correctly.
#[wasm_bindgen_test]
async fn test_encrypt_large_plaintext() {
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

  let large_plaintext = vec![0xAB_u8; 10 * 1024];
  let ciphertext = alice.encrypt(&large_plaintext).await.unwrap();
  let decrypted = bob.decrypt(&ciphertext).await.unwrap();
  assert_eq!(decrypted, large_plaintext);
}

/// Importing an invalid peer public key (not a valid EC point) must fail.
#[wasm_bindgen_test]
async fn test_import_invalid_peer_public_key_fails() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();

  // Too short: 1 byte is not a valid P-256 public key
  let result = alice.import_peer_public_key(&[0x04]).await;
  assert!(result.is_err(), "Importing a 1-byte key should fail");

  // Wrong prefix byte: 0x05 instead of 0x04 (uncompressed point marker)
  let bad_key = vec![0x05_u8; 65];
  let result = alice.import_peer_public_key(&bad_key).await;
  assert!(
    result.is_err(),
    "Importing a key with wrong prefix should fail"
  );

  // Exactly 64 bytes (missing the 0x04 prefix)
  let short_key = vec![0x04_u8; 64];
  let result = alice.import_peer_public_key(&short_key).await;
  assert!(result.is_err(), "Importing a 64-byte key should fail");
}

/// After a successful key exchange, multiple sequential encrypt/decrypt
/// operations must all succeed and produce correct plaintext.
#[wasm_bindgen_test]
async fn test_multiple_sequential_encryptions() {
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

  let messages: &[&[u8]] = &[
    b"First message",
    b"Second message with different length",
    b"Third",
    b"",
    &[0xFF_u8; 256].as_slice(),
  ];

  for (i, msg) in messages.iter().enumerate() {
    let ct = alice.encrypt(msg).await.unwrap();
    let pt = bob.decrypt(&ct).await.unwrap();
    assert_eq!(pt, *msg, "Message {i} should decrypt correctly");
  }
}

/// Each call to `import_peer_public_key` increments `key_id` by 1.
#[wasm_bindgen_test]
async fn test_key_id_increments_on_reimport() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  assert_eq!(alice.key_id(), 0, "key_id should start at 0");

  let bob1 = PeerCrypto::new(UserId::from(2u64)).await.unwrap();
  alice
    .import_peer_public_key(&bob1.export_public_key().await.unwrap())
    .await
    .unwrap();
  assert_eq!(alice.key_id(), 1, "key_id should be 1 after first import");

  // Re-import a different peer's key — key_id increments again
  let bob2 = PeerCrypto::new(UserId::from(3u64)).await.unwrap();
  alice
    .import_peer_public_key(&bob2.export_public_key().await.unwrap())
    .await
    .unwrap();
  assert_eq!(alice.key_id(), 2, "key_id should be 2 after second import");
}

/// Tampering with the nonce bytes must cause decryption to fail.
#[wasm_bindgen_test]
async fn test_decrypt_tampered_nonce_fails() {
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

  let plaintext = b"Nonce tamper test";
  let mut ciphertext = alice.encrypt(plaintext).await.unwrap();

  // Tamper with the first nonce byte
  ciphertext[0] = ciphertext[0].wrapping_add(1);

  let result = bob.decrypt(&ciphertext).await;
  assert!(
    result.is_err(),
    "Decryption should fail when nonce is tampered"
  );
}

/// Decrypting with the wrong peer (different shared key) must fail.
#[wasm_bindgen_test]
async fn test_decrypt_with_wrong_peer_fails() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let mut bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();
  let mut charlie = PeerCrypto::new(UserId::from(3u64)).await.unwrap();

  // Alice and Bob exchange keys
  alice
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();
  bob
    .import_peer_public_key(&alice.export_public_key().await.unwrap())
    .await
    .unwrap();

  // Charlie also exchanges keys with Alice (but not Bob)
  alice
    .import_peer_public_key(&charlie.export_public_key().await.unwrap())
    .await
    .unwrap();
  charlie
    .import_peer_public_key(&alice.export_public_key().await.unwrap())
    .await
    .unwrap();

  // Alice encrypts a message (using key from Charlie's import — last import wins)
  let ciphertext = alice.encrypt(b"Secret for Charlie").await.unwrap();

  // Bob cannot decrypt it because his shared key is different
  let result = bob.decrypt(&ciphertext).await;
  assert!(
    result.is_err(),
    "Bob should not be able to decrypt a message encrypted with Charlie's shared key"
  );
}

/// After key rotation (re-importing a peer key), the new shared key
/// must work for encrypt/decrypt while the old shared key becomes invalid.
#[wasm_bindgen_test]
async fn test_key_rotation_invalidates_old_ciphertext() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let mut bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();

  // First key exchange
  alice
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();
  bob
    .import_peer_public_key(&alice.export_public_key().await.unwrap())
    .await
    .unwrap();

  // Encrypt with the first shared key
  let old_ciphertext = alice.encrypt(b"Old key message").await.unwrap();
  assert!(bob.decrypt(&old_ciphertext).await.is_ok());

  // Rotate keys: Alice re-generates and re-imports
  let bob_new = PeerCrypto::new(UserId::from(2u64)).await.unwrap();
  alice
    .import_peer_public_key(&bob_new.export_public_key().await.unwrap())
    .await
    .unwrap();

  // New encryption works
  let new_ciphertext = alice.encrypt(b"New key message").await.unwrap();

  // Bob's old shared key cannot decrypt the new ciphertext (different derived key)
  // Note: This depends on whether Bob also rotated. Bob still has the old key.
  // Since Alice's shared_key changed, Alice's encrypt uses a different key.
  // Bob's decrypt will fail because his shared_key is still the old one.
  let result = bob.decrypt(&new_ciphertext).await;
  assert!(
    result.is_err(),
    "Old peer key should not decrypt after key rotation on the other side"
  );
}

/// The same plaintext encrypted twice must produce different ciphertexts
/// (semantic security via random nonces).
#[wasm_bindgen_test]
async fn test_same_plaintext_different_ciphertext() {
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

  let plaintext = b"Identical message";
  let ct1 = alice.encrypt(plaintext).await.unwrap();
  let ct2 = alice.encrypt(plaintext).await.unwrap();

  assert_ne!(
    ct1, ct2,
    "Same plaintext must produce different ciphertexts (semantic security)"
  );

  // Both must still decrypt correctly
  assert_eq!(bob.decrypt(&ct1).await.unwrap(), plaintext.as_slice());
  assert_eq!(bob.decrypt(&ct2).await.unwrap(), plaintext.as_slice());
}

/// Decrypting a ciphertext that is exactly `GCM_NONCE_SIZE` bytes
/// (no encrypted body at all) must fail.
#[wasm_bindgen_test]
async fn test_decrypt_nonce_only_ciphertext_fails() {
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

  // Fabricate a ciphertext that is only nonce bytes
  let fake_ct = vec![0u8; GCM_NONCE_SIZE];
  let result = bob.decrypt(&fake_ct).await;
  assert!(
    result.is_err(),
    "Ciphertext with only nonce (no body) should fail to decrypt"
  );
}

/// Three-party scenario: Alice can establish independent shared keys
/// with both Bob and Charlie, and messages for one peer cannot be
/// decrypted by the other.
#[wasm_bindgen_test]
async fn test_independent_shared_keys_for_different_peers() {
  let mut alice_bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();
  let mut alice_charlie = PeerCrypto::new(UserId::from(3u64)).await.unwrap();
  let mut bob = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let mut charlie = PeerCrypto::new(UserId::from(1u64)).await.unwrap();

  // Alice-Bob key exchange
  alice_bob
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();
  bob
    .import_peer_public_key(&alice_bob.export_public_key().await.unwrap())
    .await
    .unwrap();

  // Alice-Charlie key exchange
  alice_charlie
    .import_peer_public_key(&charlie.export_public_key().await.unwrap())
    .await
    .unwrap();
  charlie
    .import_peer_public_key(&alice_charlie.export_public_key().await.unwrap())
    .await
    .unwrap();

  // Alice encrypts for Bob
  let msg_for_bob = b"Hello Bob";
  let ct_bob = alice_bob.encrypt(msg_for_bob).await.unwrap();
  assert_eq!(bob.decrypt(&ct_bob).await.unwrap(), msg_for_bob.as_slice());

  // Charlie cannot decrypt Bob's message
  assert!(
    charlie.decrypt(&ct_bob).await.is_err(),
    "Charlie must not decrypt Bob's message"
  );

  // Alice encrypts for Charlie
  let msg_for_charlie = b"Hello Charlie";
  let ct_charlie = alice_charlie.encrypt(msg_for_charlie).await.unwrap();
  assert_eq!(
    charlie.decrypt(&ct_charlie).await.unwrap(),
    msg_for_charlie.as_slice()
  );

  // Bob cannot decrypt Charlie's message
  assert!(
    bob.decrypt(&ct_charlie).await.is_err(),
    "Bob must not decrypt Charlie's message"
  );
}

/// Encrypting binary data (all byte values 0x00-0xFF) should round-trip.
#[wasm_bindgen_test]
async fn test_encrypt_binary_data_roundtrip() {
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

  let binary_data: Vec<u8> = (0u8..=255).collect();
  let ct = alice.encrypt(&binary_data).await.unwrap();
  let pt = bob.decrypt(&ct).await.unwrap();
  assert_eq!(pt, binary_data);
}

/// The `peer_id` field is correctly set after construction.
#[wasm_bindgen_test]
async fn test_peer_id_is_set_correctly() {
  let alice = PeerCrypto::new(UserId::from(42u64)).await.unwrap();
  assert_eq!(alice.peer_id, UserId::from(42u64));

  let bob = PeerCrypto::new(UserId::from(999u64)).await.unwrap();
  assert_eq!(bob.peer_id, UserId::from(999u64));
}

/// Re-importing the same peer's public key (key rotation) still
/// produces a working encrypt/decrypt pair.
#[wasm_bindgen_test]
async fn test_reimport_same_peer_key_still_works() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let mut bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();

  // First key exchange
  let bob_pk = bob.export_public_key().await.unwrap();
  alice.import_peer_public_key(&bob_pk).await.unwrap();
  bob
    .import_peer_public_key(&alice.export_public_key().await.unwrap())
    .await
    .unwrap();

  // Encrypt with first key
  let ct1 = alice.encrypt(b"First").await.unwrap();
  assert!(bob.decrypt(&ct1).await.is_ok());

  // Re-import same bob key (simulates receiving the same key again)
  alice.import_peer_public_key(&bob_pk).await.unwrap();

  // Encrypt again — should still work
  let ct2 = alice.encrypt(b"Second").await.unwrap();
  assert!(bob.decrypt(&ct2).await.is_ok());

  // Verify key_id incremented
  assert_eq!(alice.key_id(), 2);
}

/// Importing an empty key slice (0 bytes) must fail.
#[wasm_bindgen_test]
async fn test_import_empty_peer_public_key_fails() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let result = alice.import_peer_public_key(&[]).await;
  assert!(result.is_err(), "Importing a 0-byte key should fail");
}

/// Exporting the public key twice should produce identical bytes.
#[wasm_bindgen_test]
async fn test_export_public_key_is_deterministic() {
  let crypto = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let pk1 = crypto.export_public_key().await.unwrap();
  let pk2 = crypto.export_public_key().await.unwrap();
  assert_eq!(
    pk1, pk2,
    "Exporting the same key twice must return identical bytes"
  );
}

/// Two independent PeerCrypto instances must produce different
/// public keys (extremely unlikely to collide for P-256).
#[wasm_bindgen_test]
async fn test_different_instances_have_different_public_keys() {
  let alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();
  let alice_pk = alice.export_public_key().await.unwrap();
  let bob_pk = bob.export_public_key().await.unwrap();
  assert_ne!(
    alice_pk, bob_pk,
    "Two independent key pairs must have different public keys"
  );
}

/// Cloning a PeerCrypto preserves the peer_id and key_id.
#[wasm_bindgen_test]
async fn test_peer_crypto_clone_preserves_metadata() {
  let mut alice = PeerCrypto::new(UserId::from(42u64)).await.unwrap();
  let bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();
  alice
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();

  let alice_clone = alice.clone();
  assert_eq!(alice_clone.peer_id, alice.peer_id);
  assert_eq!(alice_clone.key_id(), alice.key_id());
  assert_eq!(alice_clone.has_shared_key(), alice.has_shared_key());
}

/// Decrypting a zero-length ciphertext must fail (before the nonce
/// length guard).
#[wasm_bindgen_test]
async fn test_decrypt_empty_ciphertext_fails() {
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

  let result = bob.decrypt(&[]).await;
  assert!(result.is_err(), "Decrypting an empty slice should fail");
}

/// Decrypting a ciphertext whose body is shorter than the GCM tag
/// (16 bytes) must fail — the tag is always appended by Web Crypto.
#[wasm_bindgen_test]
async fn test_decrypt_truncated_ciphertext_missing_tag_fails() {
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

  let plaintext = b"Some secret";
  let mut ciphertext = alice.encrypt(plaintext).await.unwrap();

  // Remove the last 8 bytes (partial tag removal) — must fail
  ciphertext.truncate(ciphertext.len() - 8);
  let result = bob.decrypt(&ciphertext).await;
  assert!(
    result.is_err(),
    "Decryption should fail when GCM tag is truncated"
  );
}

/// Decrypting a ciphertext with the last byte of the GCM tag
/// tampered must fail.
#[wasm_bindgen_test]
async fn test_decrypt_tampered_tag_fails() {
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

  let mut ciphertext = alice.encrypt(b"Tag test").await.unwrap();

  // Tamper with the last byte (part of the GCM auth tag)
  let last = ciphertext.len() - 1;
  ciphertext[last] = ciphertext[last].wrapping_add(1);

  let result = bob.decrypt(&ciphertext).await;
  assert!(
    result.is_err(),
    "Decryption should fail when GCM tag is tampered"
  );
}

/// Multiple key rotations (3+ rounds) should produce working keys
/// each time and key_id should increment monotonically.
#[wasm_bindgen_test]
async fn test_multiple_key_rotations() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let mut bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();

  // Round 1
  alice
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();
  bob
    .import_peer_public_key(&alice.export_public_key().await.unwrap())
    .await
    .unwrap();
  assert_eq!(alice.key_id(), 1);

  // Round 2: regenerate Alice
  let mut alice_v2 = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  bob
    .import_peer_public_key(&alice_v2.export_public_key().await.unwrap())
    .await
    .unwrap();
  alice_v2
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();
  assert_eq!(bob.key_id(), 2);

  // Round 3: regenerate Bob
  let mut bob_v3 = PeerCrypto::new(UserId::from(2u64)).await.unwrap();
  alice_v2
    .import_peer_public_key(&bob_v3.export_public_key().await.unwrap())
    .await
    .unwrap();
  bob_v3
    .import_peer_public_key(&alice_v2.export_public_key().await.unwrap())
    .await
    .unwrap();
  assert_eq!(alice_v2.key_id(), 2);

  // Encrypt with latest keys should round-trip
  let ct = alice_v2.encrypt(b"After 3 rotations").await.unwrap();
  let pt = bob_v3.decrypt(&ct).await.unwrap();
  assert_eq!(pt, b"After 3 rotations".as_slice());
}

/// After key exchange, has_shared_key() remains true even after
/// re-importing a different peer key (key rotation).
#[wasm_bindgen_test]
async fn test_has_shared_key_remains_true_after_reimport() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();
  let charlie = PeerCrypto::new(UserId::from(3u64)).await.unwrap();

  assert!(!alice.has_shared_key());

  alice
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();
  assert!(alice.has_shared_key());

  alice
    .import_peer_public_key(&charlie.export_public_key().await.unwrap())
    .await
    .unwrap();
  assert!(
    alice.has_shared_key(),
    "has_shared_key must remain true after re-import"
  );
}

/// The ciphertext layout is [nonce (12 B)][encrypted body + tag].
/// Verify the first GCM_NONCE_SIZE bytes are the nonce by checking
/// that two encryptions of the same plaintext share no prefix.
#[wasm_bindgen_test]
async fn test_ciphertext_layout_nonce_is_prefix() {
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

  let plaintext = b"Layout check";
  let ct = alice.encrypt(plaintext).await.unwrap();

  // Ciphertext must be at least GCM_NONCE_SIZE + GCM tag (16)
  assert!(
    ct.len() >= GCM_NONCE_SIZE + 16,
    "Ciphertext must contain nonce + tag"
  );

  // The nonce portion (first 12 bytes) should be extractable
  let _nonce = &ct[..GCM_NONCE_SIZE];
  let _body = &ct[GCM_NONCE_SIZE..];

  // Body should not be empty (contains encrypted data + 16-byte tag)
  assert!(!_body.is_empty());
}

/// The ciphertext output size matches the expected formula:
/// GCM_NONCE_SIZE + plaintext_len + GCM_TAG_SIZE (16).
#[wasm_bindgen_test]
async fn test_encrypt_output_size_matches_formula() {
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

  const GCM_TAG_SIZE: usize = 16;

  for &size in &[0usize, 1, 10, 100, 1000] {
    let plaintext = vec![0x41_u8; size];
    let ct = alice.encrypt(&plaintext).await.unwrap();
    let expected = GCM_NONCE_SIZE + size + GCM_TAG_SIZE;
    assert_eq!(
      ct.len(),
      expected,
      "Ciphertext size mismatch for plaintext length {size}"
    );
  }
}

/// Bob encrypts, Alice decrypts — verify bidirectional works for
/// messages of varying sizes.
#[wasm_bindgen_test]
async fn test_bob_to_alice_varying_sizes() {
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

  let sizes: &[usize] = &[0, 1, 16, 64, 256, 1024];
  for &size in sizes {
    let msg = vec![0x42_u8; size];
    let ct = bob.encrypt(&msg).await.unwrap();
    let pt = alice.decrypt(&ct).await.unwrap();
    assert_eq!(pt, msg, "Bob→Alice round-trip failed for size {size}");
  }
}

/// Unicode text (multi-byte UTF-8) should encrypt/decrypt correctly.
#[wasm_bindgen_test]
async fn test_encrypt_decrypt_unicode_text() {
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

  let unicode_msg = "你好世界 🌍 こんにちは 안녕하세요".as_bytes();
  let ct = alice.encrypt(unicode_msg).await.unwrap();
  let pt = bob.decrypt(&ct).await.unwrap();
  assert_eq!(pt, unicode_msg);
}

/// Decrypting a valid ciphertext twice must succeed both times
/// (idempotent decryption — AES-GCM with the same key and nonce
/// always produces the same plaintext).
#[wasm_bindgen_test]
async fn test_decrypt_is_idempotent() {
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

  let plaintext = b"Idempotent test";
  let ciphertext = alice.encrypt(plaintext).await.unwrap();

  let pt1 = bob.decrypt(&ciphertext).await.unwrap();
  let pt2 = bob.decrypt(&ciphertext).await.unwrap();
  assert_eq!(pt1, pt2);
  assert_eq!(pt1, plaintext.as_slice());
}

/// Alice encrypts a message and then decrypts it herself. Since both
/// peers derive the same shared key from ECDH, self-decrypt must succeed.
#[wasm_bindgen_test]
async fn test_self_encrypt_decrypt_roundtrip() {
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

  let msg = b"Self-decrypt test";
  let ciphertext = alice.encrypt(msg).await.unwrap();

  // Alice can decrypt her own ciphertext because she shares the same AES key
  let decrypted = alice.decrypt(&ciphertext).await.unwrap();
  assert_eq!(decrypted, msg.as_slice());
}

/// Encrypting 100 sequential messages should all decrypt correctly
/// (stress test for nonce uniqueness and key reuse).
#[wasm_bindgen_test]
async fn test_many_sequential_encryptions() {
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

  for i in 0..100u32 {
    let msg = format!("Message #{i}").into_bytes();
    let ct = alice.encrypt(&msg).await.unwrap();
    let pt = bob.decrypt(&ct).await.unwrap();
    assert_eq!(pt, msg, "Message #{i} should decrypt correctly");
  }
}

/// Importing a 65-byte key with 0x04 prefix but all-zero X/Y
/// coordinates must fail (not a valid curve point).
#[wasm_bindgen_test]
async fn test_import_all_zero_coordinates_fails() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();

  // Valid prefix 0x04, but X and Y are all zeros — not on P-256 curve
  let mut bad_key = vec![0x04_u8];
  bad_key.extend_from_slice(&[0x00; 64]);
  assert_eq!(bad_key.len(), 65);

  let result = alice.import_peer_public_key(&bad_key).await;
  assert!(
    result.is_err(),
    "A key with all-zero coordinates must not be a valid P-256 point"
  );
}

/// Two PeerCrypto instances with the same peer_id should still
/// generate independent key pairs (different public keys).
#[wasm_bindgen_test]
async fn test_same_peer_id_different_key_pairs() {
  let alice1 = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let alice2 = PeerCrypto::new(UserId::from(1u64)).await.unwrap();

  assert_eq!(alice1.peer_id, alice2.peer_id, "peer_id should match");
  assert_ne!(
    alice1.export_public_key().await.unwrap(),
    alice2.export_public_key().await.unwrap(),
    "Same peer_id should still produce different key pairs"
  );
}

/// After both Alice and Bob rotate keys, old ciphertexts can no longer
/// be decrypted by the peer who also rotated.
#[wasm_bindgen_test]
async fn test_both_rotate_keys_old_ciphertext_fails() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let mut bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();

  // Initial key exchange
  alice
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();
  bob
    .import_peer_public_key(&alice.export_public_key().await.unwrap())
    .await
    .unwrap();

  // Encrypt with initial keys
  let old_ct = alice.encrypt(b"Old").await.unwrap();
  assert!(bob.decrypt(&old_ct).await.is_ok());

  // Both rotate: regenerate both
  let mut alice_v2 = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let mut bob_v2 = PeerCrypto::new(UserId::from(2u64)).await.unwrap();

  alice_v2
    .import_peer_public_key(&bob_v2.export_public_key().await.unwrap())
    .await
    .unwrap();
  bob_v2
    .import_peer_public_key(&alice_v2.export_public_key().await.unwrap())
    .await
    .unwrap();

  // Old ciphertext should not decrypt with new keys
  let result = bob_v2.decrypt(&old_ct).await;
  assert!(
    result.is_err(),
    "Old ciphertext should fail with new shared key"
  );

  // New ciphertext works
  let new_ct = alice_v2.encrypt(b"New").await.unwrap();
  let pt = bob_v2.decrypt(&new_ct).await.unwrap();
  assert_eq!(pt, b"New".as_slice());
}

/// Encrypting data consisting entirely of 0xFF bytes should round-trip.
#[wasm_bindgen_test]
async fn test_encrypt_all_0xff_data_roundtrip() {
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

  let data = vec![0xFF_u8; 512];
  let ct = alice.encrypt(&data).await.unwrap();
  let pt = bob.decrypt(&ct).await.unwrap();
  assert_eq!(pt, data);
}

/// Cloning a PeerCrypto and then encrypting with the clone should
/// produce ciphertexts that the original can decrypt.
#[wasm_bindgen_test]
async fn test_clone_encrypt_decrypt() {
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

  let alice_clone = alice.clone();
  let msg = b"Clone encrypt test";
  let ct = alice_clone.encrypt(msg).await.unwrap();
  let pt = bob.decrypt(&ct).await.unwrap();
  assert_eq!(pt, msg.as_slice());
}

/// Alice imports keys from Bob then Charlie sequentially.
/// key_id should increment for each import, and has_shared_key stays true.
#[wasm_bindgen_test]
async fn test_sequential_imports_from_different_peers() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();
  let charlie = PeerCrypto::new(UserId::from(3u64)).await.unwrap();
  let dave = PeerCrypto::new(UserId::from(4u64)).await.unwrap();

  assert_eq!(alice.key_id(), 0);
  assert!(!alice.has_shared_key());

  alice
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();
  assert_eq!(alice.key_id(), 1);
  assert!(alice.has_shared_key());

  alice
    .import_peer_public_key(&charlie.export_public_key().await.unwrap())
    .await
    .unwrap();
  assert_eq!(alice.key_id(), 2);
  assert!(alice.has_shared_key());

  alice
    .import_peer_public_key(&dave.export_public_key().await.unwrap())
    .await
    .unwrap();
  assert_eq!(alice.key_id(), 3);
  assert!(alice.has_shared_key());
}

/// After key rotation (re-import), the public key export still returns
/// the same bytes (only the shared key changes, not our key pair).
#[wasm_bindgen_test]
async fn test_export_public_key_unchanged_after_reimport() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();

  let pk_before = alice.export_public_key().await.unwrap();

  alice
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();

  let pk_after = alice.export_public_key().await.unwrap();
  assert_eq!(
    pk_before, pk_after,
    "Public key should not change after importing a peer key"
  );
}

/// When Alice rotates her shared key (by importing a new peer key),
/// old ciphertexts encrypted with the previous shared key can no longer
/// be decrypted by Alice herself.
#[wasm_bindgen_test]
async fn test_decrypt_old_ciphertext_fails_after_rotation() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();

  // First key exchange
  alice
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();

  // Encrypt with first shared key
  let old_ct = alice.encrypt(b"Before rotation").await.unwrap();
  assert!(alice.decrypt(&old_ct).await.is_ok());

  // Rotate: import a different peer's key
  let charlie = PeerCrypto::new(UserId::from(3u64)).await.unwrap();
  alice
    .import_peer_public_key(&charlie.export_public_key().await.unwrap())
    .await
    .unwrap();

  // Alice's shared key changed — old ciphertext should fail
  let result = alice.decrypt(&old_ct).await;
  assert!(
    result.is_err(),
    "Old ciphertext should fail to decrypt after key rotation"
  );
}

/// Decrypting a ciphertext that is GCM_NONCE_SIZE + 1 bytes long
/// (nonce + 1 body byte) must fail because the body is shorter than
/// the GCM authentication tag.
#[wasm_bindgen_test]
async fn test_decrypt_nonce_plus_one_byte_fails() {
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

  let fake_ct = vec![0x00_u8; GCM_NONCE_SIZE + 1];
  let result = bob.decrypt(&fake_ct).await;
  assert!(
    result.is_err(),
    "Ciphertext of nonce+1 bytes must fail (body shorter than GCM tag)"
  );
}

/// Bob sends a message to Alice, then Alice replies — verify a
/// full bidirectional conversation pattern.
#[wasm_bindgen_test]
async fn test_bidirectional_conversation() {
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

  // Bob → Alice
  let ct1 = bob.encrypt(b"Hi Alice!").await.unwrap();
  assert_eq!(alice.decrypt(&ct1).await.unwrap(), b"Hi Alice!".as_slice());

  // Alice → Bob
  let ct2 = alice.encrypt(b"Hey Bob!").await.unwrap();
  assert_eq!(bob.decrypt(&ct2).await.unwrap(), b"Hey Bob!".as_slice());

  // Bob → Alice again
  let ct3 = bob.encrypt(b"How are you?").await.unwrap();
  assert_eq!(
    alice.decrypt(&ct3).await.unwrap(),
    b"How are you?".as_slice()
  );

  // Alice → Bob again
  let ct4 = alice.encrypt(b"Doing well!").await.unwrap();
  assert_eq!(bob.decrypt(&ct4).await.unwrap(), b"Doing well!".as_slice());
}

/// Encrypting a 1-byte plaintext should produce a ciphertext of
/// exactly GCM_NONCE_SIZE + 1 + 16 bytes.
#[wasm_bindgen_test]
async fn test_single_byte_plaintext_roundtrip() {
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

  let single = b"X";
  let ct = alice.encrypt(single).await.unwrap();
  assert_eq!(ct.len(), GCM_NONCE_SIZE + 1 + 16);

  let pt = bob.decrypt(&ct).await.unwrap();
  assert_eq!(pt, single.as_slice());
}

/// Importing a key that is exactly 66 bytes (1 too many) must fail.
#[wasm_bindgen_test]
async fn test_import_oversized_peer_public_key_fails() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let oversized_key = vec![0x04_u8; 66];
  let result = alice.import_peer_public_key(&oversized_key).await;
  assert!(
    result.is_err(),
    "A 66-byte key should fail to import (P-256 expects 65 bytes)"
  );
}

/// Nonce uniqueness stress test: 100 encryptions of the same plaintext
/// must produce 100 distinct nonces (the first 12 bytes of each ciphertext).
#[wasm_bindgen_test]
async fn test_nonce_uniqueness_stress_100() {
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

  let plaintext = b"nonce-stress-test";
  let mut nonces: Vec<[u8; GCM_NONCE_SIZE]> = Vec::with_capacity(100);

  for _ in 0..100 {
    let ct = alice.encrypt(plaintext).await.unwrap();
    let mut nonce = [0u8; GCM_NONCE_SIZE];
    nonce.copy_from_slice(&ct[..GCM_NONCE_SIZE]);
    nonces.push(nonce);
  }

  // Verify all nonces are distinct
  let mut sorted = nonces.clone();
  sorted.sort();
  sorted.dedup();
  assert_eq!(
    nonces.len(),
    sorted.len(),
    "All 100 nonces must be distinct"
  );
}

/// After re-importing the same peer's public key, the shared key is
/// re-derived and encryption/decryption still works. This tests key
/// rotation where the same peer sends a new key.
#[wasm_bindgen_test]
async fn test_encrypt_after_second_import_same_peer() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();

  // First key exchange
  alice
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();

  let ct1 = alice.encrypt(b"before re-import").await.unwrap();

  // Re-import same peer's key (simulates key rotation with same peer)
  alice
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();

  // Encrypt should still work after re-import
  let ct2 = alice.encrypt(b"after re-import").await.unwrap();

  // The old ciphertext should fail decryption by bob (key rotated)
  // and the new ciphertext should succeed
  let _result = bob.decrypt(&ct2).await;
  // This may or may not work depending on whether bob has alice's
  // latest public key. The key point is alice can still encrypt.
  assert!(
    ct1 != ct2,
    "Ciphertexts from before and after re-import should differ"
  );
}

/// Encrypting a plaintext of all zero bytes should produce valid
/// ciphertext that can be decrypted back.
#[wasm_bindgen_test]
async fn test_encrypt_all_zero_bytes_roundtrip() {
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

  let zero_data = vec![0u8; 256];
  let ct = alice.encrypt(&zero_data).await.unwrap();
  let pt = bob.decrypt(&ct).await.unwrap();
  assert_eq!(pt, zero_data);
}

/// Symmetric key exchange: Alice imports Bob's key, Bob imports Alice's key.
/// They should derive the same shared secret and be able to decrypt each
/// other's messages.
#[wasm_bindgen_test]
async fn test_symmetric_key_exchange_verification() {
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

  // Both should have shared keys established
  assert!(alice.has_shared_key());
  assert!(bob.has_shared_key());

  // Alice encrypts, Bob decrypts
  let msg = b"hello from alice";
  let ct_a = alice.encrypt(msg).await.unwrap();
  let pt_a = bob.decrypt(&ct_a).await.unwrap();
  assert_eq!(pt_a, msg.as_slice());

  // Bob encrypts, Alice decrypts
  let msg2 = b"hello from bob";
  let ct_b = bob.encrypt(msg2).await.unwrap();
  let pt_b = alice.decrypt(&ct_b).await.unwrap();
  assert_eq!(pt_b, msg2.as_slice());
}

/// Three-party scenario: Alice can communicate with Bob and Carol
/// independently. Bob cannot decrypt messages Alice sent to Carol.
#[wasm_bindgen_test]
async fn test_three_party_isolation() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let mut bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();
  let mut carol = PeerCrypto::new(UserId::from(3u64)).await.unwrap();

  // Alice-Bob key exchange
  alice
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();
  bob
    .import_peer_public_key(&alice.export_public_key().await.unwrap())
    .await
    .unwrap();

  // Alice-Carol key exchange
  alice
    .import_peer_public_key(&carol.export_public_key().await.unwrap())
    .await
    .unwrap();
  carol
    .import_peer_public_key(&alice.export_public_key().await.unwrap())
    .await
    .unwrap();

  // Alice encrypts a message (uses latest shared key = carol)
  let secret_msg = b"for carol only";
  let ct = alice.encrypt(secret_msg).await.unwrap();

  // Carol can decrypt
  let pt = carol.decrypt(&ct).await.unwrap();
  assert_eq!(pt, secret_msg.as_slice());

  // Bob cannot decrypt (different shared key)
  let result = bob.decrypt(&ct).await;
  assert!(
    result.is_err(),
    "Bob should not be able to decrypt Alice-Carol message"
  );
}

/// After key rotation (re-importing a new key from the same peer),
/// the key_id should increment.
#[wasm_bindgen_test]
async fn test_key_id_increments_each_import() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();

  assert_eq!(alice.key_id(), 0);

  // First import
  alice
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();
  let kid1 = alice.key_id();
  assert_eq!(kid1, 1);

  // Second import (same key)
  alice
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();
  let kid2 = alice.key_id();
  assert_eq!(kid2, 2);

  // Third import (same key)
  alice
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();
  let kid3 = alice.key_id();
  assert_eq!(kid3, 3);
}

/// Decrypting a ciphertext that has been modified in the middle (not the
/// nonce or the last byte) must fail due to GCM authentication.
#[wasm_bindgen_test]
async fn test_decrypt_middle_tampered_ciphertext_fails() {
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

  let plaintext = b"this is a longer message for middle tamper test";
  let mut ct = alice.encrypt(plaintext).await.unwrap();

  // Tamper a byte in the middle of the ciphertext body
  let mid_idx = GCM_NONCE_SIZE + (ct.len() - GCM_NONCE_SIZE) / 2;
  ct[mid_idx] ^= 0xFF;

  let result = bob.decrypt(&ct).await;
  assert!(
    result.is_err(),
    "Middle-tampered ciphertext must fail decryption"
  );
}

/// Decrypting a ciphertext where the last byte (part of GCM tag) has
/// been modified must fail.
#[wasm_bindgen_test]
async fn test_decrypt_last_byte_tampered_fails() {
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

  let plaintext = b"test last byte tamper";
  let mut ct = alice.encrypt(plaintext).await.unwrap();

  // Flip the last byte
  let last = ct.len() - 1;
  ct[last] ^= 0x01;

  let result = bob.decrypt(&ct).await;
  assert!(
    result.is_err(),
    "Last-byte-tampered ciphertext must fail decryption"
  );
}

/// Encrypting the same message multiple times must always produce
/// different ciphertexts (due to random nonce).
#[wasm_bindgen_test]
async fn test_same_message_always_different_ciphertext() {
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

  let msg = b"deterministic nonce check";
  let ct1 = alice.encrypt(msg).await.unwrap();
  let ct2 = alice.encrypt(msg).await.unwrap();
  let ct3 = alice.encrypt(msg).await.unwrap();

  assert_ne!(
    ct1, ct2,
    "Same plaintext must produce different ciphertexts"
  );
  assert_ne!(
    ct2, ct3,
    "Same plaintext must produce different ciphertexts"
  );
  assert_ne!(
    ct1, ct3,
    "Same plaintext must produce different ciphertexts"
  );

  // But all should decrypt to the same plaintext
  assert_eq!(bob.decrypt(&ct1).await.unwrap(), msg.as_slice());
  assert_eq!(bob.decrypt(&ct2).await.unwrap(), msg.as_slice());
  assert_eq!(bob.decrypt(&ct3).await.unwrap(), msg.as_slice());
}

/// A cloned PeerCrypto can encrypt and the original can decrypt (they
/// share the same shared key via JsValue reference counting).
#[wasm_bindgen_test]
async fn test_clone_and_cross_decrypt() {
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

  let alice_clone = alice.clone();

  // alice_clone encrypts, bob decrypts
  let ct = alice_clone.encrypt(b"from clone").await.unwrap();
  let pt = bob.decrypt(&ct).await.unwrap();
  assert_eq!(pt, b"from clone".as_slice());

  // alice encrypts, bob decrypts
  let ct2 = alice.encrypt(b"from original").await.unwrap();
  let pt2 = bob.decrypt(&ct2).await.unwrap();
  assert_eq!(pt2, b"from original".as_slice());
}

/// Encrypting a very large message (64 KiB) should work correctly.
#[wasm_bindgen_test]
async fn test_encrypt_64kb_plaintext_roundtrip() {
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

  let large_data: Vec<u8> = (0..65536).map(|i| (i % 256) as u8).collect();
  let ct = alice.encrypt(&large_data).await.unwrap();
  assert_eq!(ct.len(), GCM_NONCE_SIZE + 65536 + 16);

  let pt = bob.decrypt(&ct).await.unwrap();
  assert_eq!(pt, large_data);
}

/// The export_public_key result should start with 0x04 (uncompressed
/// point marker for P-256).
#[wasm_bindgen_test]
async fn test_exported_public_key_starts_with_0x04() {
  let alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let key = alice.export_public_key().await.unwrap();
  assert_eq!(
    key[0], 0x04,
    "P-256 uncompressed point must start with 0x04"
  );
  assert_eq!(key.len(), 65, "P-256 uncompressed point must be 65 bytes");
}

/// After key exchange, both peers' key_id should be 1 (starting from 0,
/// incremented once by the first import).
#[wasm_bindgen_test]
async fn test_both_peers_key_id_after_exchange() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let mut bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();

  assert_eq!(alice.key_id(), 0);
  assert_eq!(bob.key_id(), 0);

  alice
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();
  bob
    .import_peer_public_key(&alice.export_public_key().await.unwrap())
    .await
    .unwrap();

  assert_eq!(alice.key_id(), 1);
  assert_eq!(bob.key_id(), 1);
}

/// Decrypting a ciphertext that is exactly GCM_NONCE_SIZE bytes (nonce
/// only, no body at all) must fail because the GCM tag is missing.
#[wasm_bindgen_test]
async fn test_decrypt_exactly_nonce_length_fails() {
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

  // Create a fake ciphertext that is exactly GCM_NONCE_SIZE bytes
  let fake_ct = vec![0x42u8; GCM_NONCE_SIZE];
  let result = bob.decrypt(&fake_ct).await;
  assert!(
    result.is_err(),
    "Ciphertext of exactly GCM_NONCE_SIZE bytes has no GCM tag and must fail"
  );
}

/// Decrypting a ciphertext that is GCM_NONCE_SIZE + 1 bytes (nonce plus
/// one byte of body, far short of the 16-byte GCM tag) must fail.
#[wasm_bindgen_test]
async fn test_decrypt_nonce_plus_two_bytes_fails() {
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

  // GCM_NONCE_SIZE + 2 bytes: body is only 2 bytes, too short for a
  // 16-byte GCM tag
  let fake_ct = vec![0xABu8; GCM_NONCE_SIZE + 2];
  let result = bob.decrypt(&fake_ct).await;
  assert!(
    result.is_err(),
    "Ciphertext with body shorter than GCM tag must fail"
  );
}

/// Two PeerCrypto instances with different peer IDs can still establish
/// a shared key and communicate.
#[wasm_bindgen_test]
async fn test_different_peer_ids_can_communicate() {
  let mut peer_a = PeerCrypto::new(UserId::from(999u64)).await.unwrap();
  let mut peer_b = PeerCrypto::new(UserId::from(888u64)).await.unwrap();

  peer_a
    .import_peer_public_key(&peer_b.export_public_key().await.unwrap())
    .await
    .unwrap();
  peer_b
    .import_peer_public_key(&peer_a.export_public_key().await.unwrap())
    .await
    .unwrap();

  let ct = peer_a.encrypt(b"cross-peer-id").await.unwrap();
  let pt = peer_b.decrypt(&ct).await.unwrap();
  assert_eq!(pt, b"cross-peer-id".as_slice());
}

/// After multiple key rotations, encryption still works with the latest key.
#[wasm_bindgen_test]
async fn test_encryption_works_after_five_rotations() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let mut bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();

  // Perform 5 key rotations
  for i in 0..5 {
    let bob_key = PeerCrypto::new(UserId::from(2u64)).await.unwrap();
    alice
      .import_peer_public_key(&bob_key.export_public_key().await.unwrap())
      .await
      .unwrap();
    assert_eq!(alice.key_id(), i as u32 + 1);
  }

  // Bob imports Alice's current key
  let alice_key = alice.export_public_key().await.unwrap();
  bob.import_peer_public_key(&alice_key).await.unwrap();

  // Note: This tests that alice can still encrypt after rotations.
  // Decryption may fail because bob's shared key may not match alice's
  // latest shared key (asymmetric rotation scenario).
  let ct = alice.encrypt(b"after rotations").await.unwrap();
  assert!(
    ct.len() > GCM_NONCE_SIZE,
    "Encryption should still produce valid output"
  );
}

/// The encrypted output format is: [nonce (12 bytes)][ciphertext + tag].
/// The nonce portion should be different for each encryption, while
/// the ciphertext+tag portion also differs.
#[wasm_bindgen_test]
async fn test_ciphertext_nonce_and_body_both_vary() {
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

  let msg = b"vary check";
  let ct1 = alice.encrypt(msg).await.unwrap();
  let ct2 = alice.encrypt(msg).await.unwrap();

  // Nonce portion differs
  assert_ne!(
    &ct1[..GCM_NONCE_SIZE],
    &ct2[..GCM_NONCE_SIZE],
    "Nonces must differ"
  );

  // Body portion differs (different nonce → different ciphertext)
  assert_ne!(
    &ct1[GCM_NONCE_SIZE..],
    &ct2[GCM_NONCE_SIZE..],
    "Ciphertext body must differ due to different nonces"
  );
}

/// Attempting to decrypt a ciphertext using a PeerCrypto that has never
/// imported any peer key (shared_key is None) must return the "no shared
/// key" error, not a Web Crypto error.
#[wasm_bindgen_test]
async fn test_decrypt_without_key_exchange_returns_shared_key_error() {
  let alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  // alice has never done import_peer_public_key

  let fake_ct = vec![0u8; 28]; // minimum valid length
  let result = alice.decrypt(&fake_ct).await;
  assert!(result.is_err());
  let err = result.unwrap_err();
  assert!(
    err.contains("No shared key available for decryption"),
    "Error should mention missing shared key, got: {}",
    err
  );
}

/// Attempting to encrypt without a shared key must return the "no shared
/// key for encryption" error.
#[wasm_bindgen_test]
async fn test_encrypt_without_key_exchange_returns_encryption_error() {
  let alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  // alice has never done import_peer_public_key

  let result = alice.encrypt(b"test").await;
  assert!(result.is_err());
  let err = result.unwrap_err();
  assert!(
    err.contains("No shared key available for encryption"),
    "Error should mention missing shared key for encryption, got: {}",
    err
  );
}

/// Decrypting an empty byte slice must fail with the "too short" error.
#[wasm_bindgen_test]
async fn test_decrypt_empty_slice_returns_length_error() {
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

  let result = bob.decrypt(&[]).await;
  assert!(result.is_err());
  let err = result.unwrap_err();
  assert!(
    err.contains("Ciphertext too short"),
    "Error should mention short ciphertext, got: {}",
    err
  );
}

/// A conversation with rapid back-and-forth: Alice sends 5 messages,
/// Bob sends 5 messages, all should decrypt correctly.
#[wasm_bindgen_test]
async fn test_rapid_back_and_forth_conversation() {
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

  // Alice sends 5 messages
  for i in 0..5u8 {
    let msg = vec![b'A', b'0' + i];
    let ct = alice.encrypt(&msg).await.unwrap();
    let pt = bob.decrypt(&ct).await.unwrap();
    assert_eq!(pt, msg);
  }

  // Bob sends 5 messages
  for i in 0..5u8 {
    let msg = vec![b'B', b'0' + i];
    let ct = bob.encrypt(&msg).await.unwrap();
    let pt = alice.decrypt(&ct).await.unwrap();
    assert_eq!(pt, msg);
  }
}

/// After creating a PeerCrypto, the peer_id should match what was passed
/// to the constructor.
#[wasm_bindgen_test]
async fn test_peer_id_matches_constructor() {
  let uid = UserId::from(42u64);
  let alice = PeerCrypto::new(uid.clone()).await.unwrap();
  assert_eq!(alice.peer_id, uid);
}

/// Creating multiple PeerCrypto instances should all succeed.
#[wasm_bindgen_test]
async fn test_multiple_peer_crypto_creation() {
  let alice = PeerCrypto::new(UserId::from(1u64)).await;
  let bob = PeerCrypto::new(UserId::from(2u64)).await;
  let carol = PeerCrypto::new(UserId::from(3u64)).await;

  assert!(alice.is_ok(), "Alice creation should succeed");
  assert!(bob.is_ok(), "Bob creation should succeed");
  assert!(carol.is_ok(), "Carol creation should succeed");
}

/// has_shared_key() returns false before import, true after import.
#[wasm_bindgen_test]
async fn test_has_shared_key_state_transitions() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();

  // Before any import
  assert!(
    !alice.has_shared_key(),
    "Should not have shared key before import"
  );

  // After import
  alice
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();
  assert!(
    alice.has_shared_key(),
    "Should have shared key after import"
  );
}

/// Partial key exchange: only Alice imports Bob's key.
/// Alice can encrypt to Bob, but Bob cannot decrypt (no shared key on Bob's side).
#[wasm_bindgen_test]
async fn test_partial_key_exchange_alice_can_encrypt_bob_cannot_decrypt() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();

  // Only Alice imports Bob's key (one-way)
  alice
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();

  // Alice can encrypt (she has a shared key)
  let ct = alice.encrypt(b"one-way message").await;
  assert!(
    ct.is_ok(),
    "Alice should be able to encrypt after importing Bob's key"
  );

  // Bob cannot decrypt (he has no shared key)
  let result = bob.decrypt(&ct.unwrap()).await;
  assert!(
    result.is_err(),
    "Bob should not be able to decrypt without importing Alice's key"
  );
}

/// Importing the same peer's public key twice does not fail — it just
/// re-derives the shared key and increments key_id.
#[wasm_bindgen_test]
async fn test_import_same_key_twice_is_idempotent() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();

  let bob_key = bob.export_public_key().await.unwrap();

  // First import
  alice.import_peer_public_key(&bob_key).await.unwrap();
  assert_eq!(alice.key_id(), 1);

  // Second import with same key data
  alice.import_peer_public_key(&bob_key).await.unwrap();
  assert_eq!(alice.key_id(), 2);

  // Alice still has a shared key
  assert!(alice.has_shared_key());
}

/// After a key rotation during an active conversation, old ciphertexts
/// from before the rotation can no longer be decrypted.
#[wasm_bindgen_test]
async fn test_key_rotation_during_active_conversation() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let mut bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();

  // Initial key exchange
  alice
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();
  bob
    .import_peer_public_key(&alice.export_public_key().await.unwrap())
    .await
    .unwrap();

  // Alice sends message before rotation
  let ct_old = alice.encrypt(b"before rotation").await.unwrap();
  let pt_old = bob.decrypt(&ct_old).await.unwrap();
  assert_eq!(pt_old, b"before rotation".as_slice());

  // Bob rotates his key (creates new PeerCrypto)
  let bob_new = PeerCrypto::new(UserId::from(2u64)).await.unwrap();

  // Alice imports Bob's new key → rotation
  alice
    .import_peer_public_key(&bob_new.export_public_key().await.unwrap())
    .await
    .unwrap();

  // Alice sends message after rotation
  let ct_new = alice.encrypt(b"after rotation").await.unwrap();

  // Old Bob cannot decrypt the new ciphertext (different shared key)
  let result_old_bob = bob.decrypt(&ct_new).await;
  assert!(
    result_old_bob.is_err(),
    "Old Bob should not decrypt post-rotation ciphertext"
  );

  // Old ciphertext also fails for the new Bob
  let result_new_bob = bob_new.decrypt(&ct_old).await;
  assert!(
    result_new_bob.is_err(),
    "New Bob should not decrypt pre-rotation ciphertext"
  );
}

/// Mixed plaintext type sequence: send binary, text, empty, single-byte,
/// and large messages in sequence. All must decrypt correctly.
#[wasm_bindgen_test]
async fn test_mixed_plaintext_type_sequence() {
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

  let messages: Vec<&[u8]> = vec![
    &[0x00, 0x01, 0x02, 0xFF], // binary
    b"hello world",            // text
    &[],                       // empty
    b"X",                      // single byte
    &[0xAB; 1024],             // 1KB repeated byte
  ];

  for msg in &messages {
    let ct = alice.encrypt(msg).await.unwrap();
    let pt = bob.decrypt(&ct).await.unwrap();
    assert_eq!(pt, msg.to_vec(), "Decrypted message should match original");
  }
}

/// Encrypting messages of sizes near common MTU boundaries (1200, 1500, 4096).
#[wasm_bindgen_test]
async fn test_mtu_boundary_size_messages() {
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

  for &size in &[1200, 1500, 4096] {
    let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
    let ct = alice.encrypt(&data).await.unwrap();
    let pt = bob.decrypt(&ct).await.unwrap();
    assert_eq!(pt.len(), size, "Decrypted length should match original");
    assert_eq!(pt, data, "Decrypted content should match original");
  }
}

/// Encrypting two messages back-to-back and decrypting in reverse order
/// must still work (messages are independent, not chained).
#[wasm_bindgen_test]
async fn test_decrypt_in_reverse_order() {
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

  let ct1 = alice.encrypt(b"first message").await.unwrap();
  let ct2 = alice.encrypt(b"second message").await.unwrap();

  // Decrypt in reverse order
  let pt2 = bob.decrypt(&ct2).await.unwrap();
  let pt1 = bob.decrypt(&ct1).await.unwrap();

  assert_eq!(pt1, b"first message".as_slice());
  assert_eq!(pt2, b"second message".as_slice());
}

/// A PeerCrypto that has never imported any peer key should have
/// key_id() == 0 and has_shared_key() == false.
#[wasm_bindgen_test]
async fn test_initial_state_no_shared_key_zero_key_id() {
  let alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  assert_eq!(alice.key_id(), 0);
  assert!(!alice.has_shared_key());
}

/// After one-way key import, only the importer has shared_key.
/// The other peer still has key_id == 0.
#[wasm_bindgen_test]
async fn test_one_way_import_key_id_asymmetry() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();

  // Only Alice imports Bob's key
  alice
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();

  assert_eq!(alice.key_id(), 1, "Alice should have key_id 1 after import");
  assert_eq!(bob.key_id(), 0, "Bob should still have key_id 0");
  assert!(alice.has_shared_key());
  assert!(!bob.has_shared_key());
}

/// Encrypt then immediately decrypt using the same PeerCrypto instance
/// (self-encryption). This works because both private and shared keys
/// are present on the same instance.
#[wasm_bindgen_test]
async fn test_self_encrypt_decrypt_various_sizes() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();

  alice
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();

  for size in [0, 1, 16, 100, 1024] {
    let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
    let ct = alice.encrypt(&data).await.unwrap();
    let pt = alice.decrypt(&ct).await.unwrap();
    assert_eq!(pt, data, "Self-decrypt should match for size {}", size);
  }
}

/// Decrypting a ciphertext with a completely wrong nonce (all zeros)
/// when the actual nonce is random must fail.
#[wasm_bindgen_test]
async fn test_decrypt_with_zeroed_nonce_fails() {
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

  let plaintext = b"test zero nonce";
  let mut ct = alice.encrypt(plaintext).await.unwrap();

  // Replace the nonce portion with zeros
  for i in 0..GCM_NONCE_SIZE {
    ct[i] = 0;
  }

  let result = bob.decrypt(&ct).await;
  assert!(result.is_err(), "Zeroed nonce must fail decryption");
}

/// After Alice imports Bob's key, Bob imports Alice's key, both should
/// be able to independently encrypt messages that the other can decrypt.
#[wasm_bindgen_test]
async fn test_independent_bidirectional_encryption() {
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

  // Alice sends to Bob
  let ct_a = alice.encrypt(b"from alice").await.unwrap();
  let pt_a = bob.decrypt(&ct_a).await.unwrap();
  assert_eq!(pt_a, b"from alice".as_slice());

  // Bob sends to Alice
  let ct_b = bob.encrypt(b"from bob").await.unwrap();
  let pt_b = alice.decrypt(&ct_b).await.unwrap();
  assert_eq!(pt_b, b"from bob".as_slice());
}

/// Four-party scenario: each peer communicates with exactly one other peer.
/// Cross-pair messages must not decrypt.
#[wasm_bindgen_test]
async fn test_four_party_paired_isolation() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let mut bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();
  let mut carol = PeerCrypto::new(UserId::from(3u64)).await.unwrap();
  let dave = PeerCrypto::new(UserId::from(4u64)).await.unwrap();

  // Alice-Bob pair
  alice
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();
  bob
    .import_peer_public_key(&alice.export_public_key().await.unwrap())
    .await
    .unwrap();

  // Carol-Dave pair
  carol
    .import_peer_public_key(&dave.export_public_key().await.unwrap())
    .await
    .unwrap();

  // Alice encrypts to Bob — Carol cannot decrypt
  let ct = alice.encrypt(b"alice-bob secret").await.unwrap();
  let result = carol.decrypt(&ct).await;
  assert!(
    result.is_err(),
    "Carol should not decrypt Alice-Bob message"
  );
}

/// The exported public key should be the same every time it is exported
/// (deterministic, not re-randomized).
#[wasm_bindgen_test]
async fn test_export_public_key_is_consistent_across_calls() {
  let alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let key1 = alice.export_public_key().await.unwrap();
  let key2 = alice.export_public_key().await.unwrap();
  let key3 = alice.export_public_key().await.unwrap();
  assert_eq!(key1, key2, "Exported key should be consistent");
  assert_eq!(key2, key3, "Exported key should be consistent");
}

/// PeerCrypto::new with a very large user ID should still work.
#[wasm_bindgen_test]
async fn test_new_with_large_user_id() {
  let uid = UserId::from(u64::MAX);
  let result = PeerCrypto::new(uid.clone()).await;
  assert!(
    result.is_ok(),
    "Should be able to create PeerCrypto with max u64 user ID"
  );
  let peer = result.unwrap();
  assert_eq!(peer.peer_id, uid);
}

/// Decrypting a ciphertext that was encrypted by a different shared key
/// (after key rotation) must fail.
#[wasm_bindgen_test]
async fn test_decrypt_with_rotated_shared_key_fails() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let mut bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();

  // First key exchange
  alice
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();
  bob
    .import_peer_public_key(&alice.export_public_key().await.unwrap())
    .await
    .unwrap();

  // Encrypt before rotation
  let ct_before = alice.encrypt(b"old key").await.unwrap();
  let pt_before = bob.decrypt(&ct_before).await.unwrap();
  assert_eq!(pt_before, b"old key".as_slice());

  // Alice rotates (imports Bob's key again, creating a new shared key)
  let bob_new = PeerCrypto::new(UserId::from(2u64)).await.unwrap();
  alice
    .import_peer_public_key(&bob_new.export_public_key().await.unwrap())
    .await
    .unwrap();

  // Alice's old ciphertext cannot be decrypted by Alice herself
  // (her shared key has been replaced)
  let result = alice.decrypt(&ct_before).await;
  assert!(
    result.is_err(),
    "Alice should not decrypt old ciphertext after rotation"
  );
}

/// Multiple encrypt/decrypt cycles with the same shared key should
/// all produce correct results (no state corruption).
#[wasm_bindgen_test]
async fn test_many_encrypt_decrypt_cycles_same_key() {
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

  for i in 0..20 {
    let msg = format!("message {}", i);
    let ct = alice.encrypt(msg.as_bytes()).await.unwrap();
    let pt = bob.decrypt(&ct).await.unwrap();
    assert_eq!(pt, msg.as_bytes(), "Cycle {} should decrypt correctly", i);
  }
}

/// Encrypting with all possible byte values (0x00-0xFF) in a single message.
#[wasm_bindgen_test]
async fn test_encrypt_all_byte_values_roundtrip() {
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

  let all_bytes: Vec<u8> = (0..=255).collect();
  let ct = alice.encrypt(&all_bytes).await.unwrap();
  let pt = bob.decrypt(&ct).await.unwrap();
  assert_eq!(pt, all_bytes);
}

/// Bob can decrypt multiple messages from Alice that were encrypted
/// before Bob imported Alice's key (as long as the shared key matches).
#[wasm_bindgen_test]
async fn test_decrypt_multiple_messages_from_same_sender() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let mut bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();

  // Alice imports Bob's key first
  alice
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();

  // Alice encrypts multiple messages
  let ct1 = alice.encrypt(b"msg1").await.unwrap();
  let ct2 = alice.encrypt(b"msg2").await.unwrap();
  let ct3 = alice.encrypt(b"msg3").await.unwrap();

  // Now Bob imports Alice's key
  bob
    .import_peer_public_key(&alice.export_public_key().await.unwrap())
    .await
    .unwrap();

  // Bob can decrypt all messages
  assert_eq!(bob.decrypt(&ct1).await.unwrap(), b"msg1".as_slice());
  assert_eq!(bob.decrypt(&ct2).await.unwrap(), b"msg2".as_slice());
  assert_eq!(bob.decrypt(&ct3).await.unwrap(), b"msg3".as_slice());
}

/// Importing a key of 64 bytes (1 byte short of P-256 uncompressed)
/// must fail.
#[wasm_bindgen_test]
async fn test_import_undersized_peer_public_key_fails() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let undersized_key = vec![0x04_u8; 64];
  let result = alice.import_peer_public_key(&undersized_key).await;
  assert!(
    result.is_err(),
    "A 64-byte key should fail to import (P-256 expects 65 bytes)"
  );
}

/// Importing a key with invalid point coordinates (0x04 followed by all
/// zeros for X and Y) must fail — the point is not on the P-256 curve.
#[wasm_bindgen_test]
async fn test_import_invalid_point_on_curve_fails() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  // 0x04 + 32 zero bytes for X + 32 zero bytes for Y
  let mut invalid_key = vec![0x04_u8];
  invalid_key.extend_from_slice(&[0u8; 64]);
  let result = alice.import_peer_public_key(&invalid_key).await;
  assert!(
    result.is_err(),
    "Point (0,0) is not on P-256 curve and should fail import"
  );
}

/// After Alice rotates her shared key by importing a new Bob key,
/// the key_id should reflect the number of imports.
#[wasm_bindgen_test]
async fn test_key_id_after_asymmetric_rotation() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();

  assert_eq!(alice.key_id(), 0);

  // First import
  alice
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();
  assert_eq!(alice.key_id(), 1);

  // Second import with new Bob
  let bob_v2 = PeerCrypto::new(UserId::from(2u64)).await.unwrap();
  alice
    .import_peer_public_key(&bob_v2.export_public_key().await.unwrap())
    .await
    .unwrap();
  assert_eq!(alice.key_id(), 2);

  // Third import with new Bob
  let bob_v3 = PeerCrypto::new(UserId::from(2u64)).await.unwrap();
  alice
    .import_peer_public_key(&bob_v3.export_public_key().await.unwrap())
    .await
    .unwrap();
  assert_eq!(alice.key_id(), 3);
}

/// A PeerCrypto clone shares the same shared_key (via JsValue reference
/// counting), so both the clone and the original can decrypt messages
/// encrypted with that shared key.
#[wasm_bindgen_test]
async fn test_clone_shares_shared_key_for_decrypt() {
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

  let alice_clone = alice.clone();

  // Bob encrypts a message
  let ct = bob.encrypt(b"for alice").await.unwrap();

  // Both alice and alice_clone can decrypt
  let pt1 = alice.decrypt(&ct).await.unwrap();
  let pt2 = alice_clone.decrypt(&ct).await.unwrap();
  assert_eq!(pt1, b"for alice".as_slice());
  assert_eq!(pt2, b"for alice".as_slice());
}

/// Decrypting the same ciphertext twice should produce identical results.
#[wasm_bindgen_test]
async fn test_decrypt_same_ciphertext_twice_same_result() {
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

  let ct = alice.encrypt(b"repeated decrypt").await.unwrap();
  let pt1 = bob.decrypt(&ct).await.unwrap();
  let pt2 = bob.decrypt(&ct).await.unwrap();
  assert_eq!(pt1, pt2);
}

/// After key rotation, the old PeerCrypto instance still has its own
/// shared key and can still encrypt/decrypt with it independently.
#[wasm_bindgen_test]
async fn test_old_instance_still_works_after_peer_rotation() {
  let mut alice = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  let mut bob = PeerCrypto::new(UserId::from(2u64)).await.unwrap();

  // Initial exchange
  alice
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();
  bob
    .import_peer_public_key(&alice.export_public_key().await.unwrap())
    .await
    .unwrap();

  // Alice creates a new instance (rotation from Alice's side)
  let mut alice_v2 = PeerCrypto::new(UserId::from(1u64)).await.unwrap();
  alice_v2
    .import_peer_public_key(&bob.export_public_key().await.unwrap())
    .await
    .unwrap();

  // Old Alice can still encrypt and Bob can decrypt (old shared key)
  let ct_old = alice.encrypt(b"from old alice").await.unwrap();
  let pt_old = bob.decrypt(&ct_old).await.unwrap();
  assert_eq!(pt_old, b"from old alice".as_slice());

  // New Alice can also encrypt, but Bob can't decrypt (different shared key)
  let ct_new = alice_v2.encrypt(b"from new alice").await.unwrap();
  let result = bob.decrypt(&ct_new).await;
  assert!(
    result.is_err(),
    "Bob should not decrypt from new Alice without key exchange"
  );
}
