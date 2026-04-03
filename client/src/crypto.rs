//! E2EE (End-to-End Encryption) Module
//!
//! Implemented using Web Crypto API:
//! - ECDH P-256 key exchange
//! - HKDF for deriving AES-256 keys
//! - AES-256-GCM encryption/decryption
//!
//! Flow:
//! 1. When DataChannel opens, both parties generate ECDH key pairs and exchange public keys
//! 2. After receiving the peer's public key, derive a shared AES-256 key via ECDH + HKDF
//! 3. Encrypt messages with the shared key when sending, decrypt when receiving
//!
//! ## Testing Note
//!
//! Unit tests are not included in this module because all functions depend on
//! `web_sys::SubtleCrypto` (Web Crypto API), which requires a full browser or
//! wasm-bindgen-test runner with crypto support. These functions are covered by
//! integration / E2E tests instead.

use std::cell::RefCell;
use std::collections::HashMap;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

/// AES-GCM IV length (12 bytes = 96 bits, NIST recommended value)
const IV_LENGTH: usize = 12;

/// HKDF salt (fixed, used for key derivation)
const HKDF_SALT: &[u8] = b"webrtc-chat-e2ee-salt-v1";

/// HKDF info (fixed, used for key derivation context binding)
const HKDF_INFO: &[u8] = b"webrtc-chat-aes256gcm";

// ============================================================================
// Global State (WASM single-threaded safe)
// ============================================================================

thread_local! {
  /// Local ECDH key pair (public key raw bytes, CryptoKeyPair)
  static LOCAL_KEY_PAIR: RefCell<Option<LocalKeyPair>> = const { RefCell::new(None) };

  /// Shared AES-256 key for each peer (remote_user_id -> CryptoKey)
  static SHARED_KEYS: RefCell<HashMap<String, web_sys::CryptoKey>> =
    RefCell::new(HashMap::new());
}

/// Local key pair
struct LocalKeyPair {
  /// Public key raw bytes (sent to peer)
  pub public_key_raw: Vec<u8>,
  /// ECDH private key (used for deriving shared key)
  pub private_key: web_sys::CryptoKey,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get SubtleCrypto instance
fn subtle() -> Result<web_sys::SubtleCrypto, String> {
  let window = web_sys::window().ok_or("Failed to get window")?;
  let crypto = window.crypto().map_err(|_| "Failed to get Crypto")?;
  Ok(crypto.subtle())
}

/// Generate cryptographically secure random bytes
fn random_bytes(len: usize) -> Result<Vec<u8>, String> {
  let mut buf = vec![0u8; len];
  let window = web_sys::window().ok_or("Failed to get window")?;
  let crypto = window.crypto().map_err(|_| "Failed to get Crypto")?;
  crypto
    .get_random_values_with_u8_array(&mut buf)
    .map_err(|e| format!("getRandomValues failed: {e:?}"))?;
  Ok(buf)
}

/// Convert `Vec<u8>` to `js_sys::Uint8Array`
fn to_uint8_array(data: &[u8]) -> js_sys::Uint8Array {
  let arr = js_sys::Uint8Array::new_with_length(data.len() as u32);
  arr.copy_from(data);
  arr
}

/// Convert `ArrayBuffer` to `Vec<u8>`
fn array_buffer_to_vec(buf: &JsValue) -> Vec<u8> {
  let arr = js_sys::Uint8Array::new(buf);
  arr.to_vec()
}

// ============================================================================
// ECDH Key Exchange
// ============================================================================

/// Generate local ECDH P-256 key pair
///
/// Returns the public key raw bytes (65 bytes, uncompressed format) for sending to peer.
pub async fn generate_key_pair() -> Result<Vec<u8>, String> {
  let subtle = subtle()?;

  // Construct ECDH key generation parameters
  let algorithm = js_sys::Object::new();
  js_sys::Reflect::set(&algorithm, &"name".into(), &"ECDH".into())
    .map_err(|e| format!("Failed to set name: {e:?}"))?;
  js_sys::Reflect::set(&algorithm, &"namedCurve".into(), &"P-256".into())
    .map_err(|e| format!("Failed to set namedCurve: {e:?}"))?;

  // generateKey(algorithm, extractable, keyUsages)
  let key_usages = js_sys::Array::new();
  key_usages.push(&"deriveKey".into());
  key_usages.push(&"deriveBits".into());

  let key_pair_promise = subtle
    .generate_key_with_object(&algorithm, true, &key_usages)
    .map_err(|e| format!("generateKey failed: {e:?}"))?;

  let key_pair_js = JsFuture::from(key_pair_promise)
    .await
    .map_err(|e| format!("generateKey await failed: {e:?}"))?;

  // Extract public key and private key
  let public_key: web_sys::CryptoKey = js_sys::Reflect::get(&key_pair_js, &"publicKey".into())
    .map_err(|e| format!("Failed to get publicKey: {e:?}"))?
    .dyn_into()
    .map_err(|_| "publicKey type conversion failed")?;

  let private_key: web_sys::CryptoKey = js_sys::Reflect::get(&key_pair_js, &"privateKey".into())
    .map_err(|e| format!("Failed to get privateKey: {e:?}"))?
    .dyn_into()
    .map_err(|_| "privateKey type conversion failed")?;

  // Export public key as raw format
  let export_promise = subtle
    .export_key("raw", &public_key)
    .map_err(|e| format!("exportKey failed: {e:?}"))?;

  let raw_key = JsFuture::from(export_promise)
    .await
    .map_err(|e| format!("exportKey await failed: {e:?}"))?;

  let public_key_raw = array_buffer_to_vec(&raw_key);

  // Store to global state
  LOCAL_KEY_PAIR.with(|kp| {
    *kp.borrow_mut() = Some(LocalKeyPair {
      public_key_raw: public_key_raw.clone(),
      private_key,
    });
  });

  web_sys::console::log_1(
    &format!(
      "E2EE: Generated ECDH key pair, public_key_length={} bytes",
      public_key_raw.len()
    )
    .into(),
  );

  Ok(public_key_raw)
}

/// Get local public key raw bytes (if generated)
pub fn get_local_public_key() -> Option<Vec<u8>> {
  LOCAL_KEY_PAIR.with(|kp| kp.borrow().as_ref().map(|k| k.public_key_raw.clone()))
}

/// Process received peer public key and derive shared AES-256 key
///
/// Uses ECDH to derive shared secret, then derives AES-256-GCM key via HKDF-SHA256.
pub async fn derive_shared_key(
  remote_user_id: &str,
  remote_public_key_raw: &[u8],
) -> Result<(), String> {
  let subtle = subtle()?;

  // Get local private key
  let private_key = LOCAL_KEY_PAIR.with(|kp| kp.borrow().as_ref().map(|k| k.private_key.clone()));
  let private_key = private_key.ok_or("Local key pair not yet generated")?;

  // Import peer public key
  let import_algorithm = js_sys::Object::new();
  js_sys::Reflect::set(&import_algorithm, &"name".into(), &"ECDH".into())
    .map_err(|e| format!("Failed to set name: {e:?}"))?;
  js_sys::Reflect::set(&import_algorithm, &"namedCurve".into(), &"P-256".into())
    .map_err(|e| format!("Failed to set namedCurve: {e:?}"))?;

  let remote_key_data = to_uint8_array(remote_public_key_raw);
  let import_promise = subtle
    .import_key_with_object(
      "raw",
      &remote_key_data.buffer(),
      &import_algorithm,
      true,
      &js_sys::Array::new(), // Public key doesn't need keyUsages
    )
    .map_err(|e| format!("importKey failed: {e:?}"))?;

  let remote_public_key: web_sys::CryptoKey = JsFuture::from(import_promise)
    .await
    .map_err(|e| format!("importKey await failed: {e:?}"))?
    .dyn_into()
    .map_err(|_| "importKey type conversion failed")?;

  // Use ECDH deriveBits to get shared secret
  let derive_algorithm = js_sys::Object::new();
  js_sys::Reflect::set(&derive_algorithm, &"name".into(), &"ECDH".into())
    .map_err(|e| format!("Failed to set name: {e:?}"))?;
  js_sys::Reflect::set(&derive_algorithm, &"public".into(), &remote_public_key)
    .map_err(|e| format!("Failed to set public: {e:?}"))?;

  let derive_bits_promise = subtle
    .derive_bits_with_object(&derive_algorithm, &private_key, 256)
    .map_err(|e| format!("deriveBits failed: {e:?}"))?;

  let shared_secret = JsFuture::from(derive_bits_promise)
    .await
    .map_err(|e| format!("deriveBits await failed: {e:?}"))?;

  // Import shared secret as HKDF key material
  let hkdf_import_promise = subtle
    .import_key_with_str("raw", shared_secret.unchecked_ref(), "HKDF", false, &{
      let arr = js_sys::Array::new();
      arr.push(&"deriveKey".into());
      arr.into()
    })
    .map_err(|e| format!("HKDF importKey failed: {e:?}"))?;

  let hkdf_key: web_sys::CryptoKey = JsFuture::from(hkdf_import_promise)
    .await
    .map_err(|e| format!("HKDF importKey await failed: {e:?}"))?
    .dyn_into()
    .map_err(|_| "HKDF importKey type conversion failed")?;

  // Derive AES-256-GCM key using HKDF
  let hkdf_params = js_sys::Object::new();
  js_sys::Reflect::set(&hkdf_params, &"name".into(), &"HKDF".into())
    .map_err(|e| format!("Failed to set name: {e:?}"))?;
  js_sys::Reflect::set(&hkdf_params, &"hash".into(), &"SHA-256".into())
    .map_err(|e| format!("Failed to set hash: {e:?}"))?;
  js_sys::Reflect::set(
    &hkdf_params,
    &"salt".into(),
    &to_uint8_array(HKDF_SALT).buffer(),
  )
  .map_err(|e| format!("Failed to set salt: {e:?}"))?;
  js_sys::Reflect::set(
    &hkdf_params,
    &"info".into(),
    &to_uint8_array(HKDF_INFO).buffer(),
  )
  .map_err(|e| format!("Failed to set info: {e:?}"))?;

  let aes_algorithm = js_sys::Object::new();
  js_sys::Reflect::set(&aes_algorithm, &"name".into(), &"AES-GCM".into())
    .map_err(|e| format!("Failed to set name: {e:?}"))?;
  js_sys::Reflect::set(&aes_algorithm, &"length".into(), &256.into())
    .map_err(|e| format!("Failed to set length: {e:?}"))?;

  let aes_key_usages = js_sys::Array::new();
  aes_key_usages.push(&"encrypt".into());
  aes_key_usages.push(&"decrypt".into());

  let derive_key_promise = subtle
    .derive_key_with_object_and_object(
      &hkdf_params,
      &hkdf_key,
      &aes_algorithm,
      false,
      &aes_key_usages,
    )
    .map_err(|e| format!("deriveKey failed: {e:?}"))?;

  let aes_key: web_sys::CryptoKey = JsFuture::from(derive_key_promise)
    .await
    .map_err(|e| format!("deriveKey await failed: {e:?}"))?
    .dyn_into()
    .map_err(|_| "deriveKey type conversion failed")?;

  // Store shared key
  let remote_id = remote_user_id.to_string();
  SHARED_KEYS.with(|keys| {
    keys.borrow_mut().insert(remote_id.clone(), aes_key);
  });

  web_sys::console::log_1(&format!("E2EE: Established shared key with {remote_id}").into());

  Ok(())
}

/// Check if shared key has been established with specified user
pub fn has_shared_key(remote_user_id: &str) -> bool {
  SHARED_KEYS.with(|keys| keys.borrow().contains_key(remote_user_id))
}

/// Remove shared key for specified user (called when disconnecting)
pub fn remove_shared_key(remote_user_id: &str) {
  SHARED_KEYS.with(|keys| {
    keys.borrow_mut().remove(remote_user_id);
  });
  web_sys::console::log_1(&format!("E2EE: Removed shared key with {remote_user_id}").into());
}

// ============================================================================
// AES-256-GCM Encryption/Decryption
// ============================================================================

/// Encrypt data using AES-256-GCM
///
/// Returns `(iv, ciphertext)` where ciphertext includes the GCM authentication tag.
pub async fn encrypt(remote_user_id: &str, plaintext: &[u8]) -> Result<(Vec<u8>, Vec<u8>), String> {
  let aes_key = SHARED_KEYS.with(|keys| keys.borrow().get(remote_user_id).cloned());
  let aes_key = aes_key.ok_or_else(|| format!("Shared key not found for {remote_user_id}"))?;

  let subtle = subtle()?;

  // Generate random IV
  let iv = random_bytes(IV_LENGTH)?;

  // Construct AES-GCM parameters
  let algorithm = js_sys::Object::new();
  js_sys::Reflect::set(&algorithm, &"name".into(), &"AES-GCM".into())
    .map_err(|e| format!("Failed to set name: {e:?}"))?;
  js_sys::Reflect::set(&algorithm, &"iv".into(), &to_uint8_array(&iv).buffer())
    .map_err(|e| format!("Failed to set iv: {e:?}"))?;

  let plaintext_arr = to_uint8_array(plaintext);
  let encrypt_promise = subtle
    .encrypt_with_object_and_buffer_source(&algorithm, &aes_key, &plaintext_arr)
    .map_err(|e| format!("encrypt failed: {e:?}"))?;

  let ciphertext_buf = JsFuture::from(encrypt_promise)
    .await
    .map_err(|e| format!("encrypt await failed: {e:?}"))?;

  let ciphertext = array_buffer_to_vec(&ciphertext_buf);

  Ok((iv, ciphertext))
}

/// Decrypt data using AES-256-GCM
pub async fn decrypt(
  remote_user_id: &str,
  iv: &[u8],
  ciphertext: &[u8],
) -> Result<Vec<u8>, String> {
  let aes_key = SHARED_KEYS.with(|keys| keys.borrow().get(remote_user_id).cloned());
  let aes_key = aes_key.ok_or_else(|| format!("Shared key not found for {remote_user_id}"))?;

  let subtle = subtle()?;

  // Construct AES-GCM parameters
  let algorithm = js_sys::Object::new();
  js_sys::Reflect::set(&algorithm, &"name".into(), &"AES-GCM".into())
    .map_err(|e| format!("Failed to set name: {e:?}"))?;
  js_sys::Reflect::set(&algorithm, &"iv".into(), &to_uint8_array(iv).buffer())
    .map_err(|e| format!("Failed to set iv: {e:?}"))?;

  let ciphertext_arr = to_uint8_array(ciphertext);
  let decrypt_promise = subtle
    .decrypt_with_object_and_buffer_source(&algorithm, &aes_key, &ciphertext_arr)
    .map_err(|e| format!("decrypt failed: {e:?}"))?;

  let plaintext_buf = JsFuture::from(decrypt_promise)
    .await
    .map_err(|e| format!("decrypt await failed (ciphertext may be tampered): {e:?}"))?;

  Ok(array_buffer_to_vec(&plaintext_buf))
}
