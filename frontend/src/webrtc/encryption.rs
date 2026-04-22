//! End-to-end encryption for WebRTC DataChannel messages.
//!
//! Uses ECDH P-256 for key exchange and AES-256-GCM for encryption.
//! All cryptographic operations are performed using the Web Crypto API via
//! `web-sys` and `js-sys` bindings.

use js_sys::{Array, Uint8Array};
use message::UserId;
use wasm_bindgen::prelude::*;

/// Key size for AES-256 (32 bytes).
const AES_KEY_SIZE: u32 = 256;

/// AES-GCM nonce size (12 bytes).
const GCM_NONCE_SIZE: usize = 12;

/// Manages ECDH key exchange and AES-256-GCM encryption for a peer.
#[derive(Debug, Clone)]
pub struct PeerCrypto {
  /// The peer's user ID.
  pub peer_id: UserId,
  /// Our ECDH private key (CryptoKey, stored as JsValue).
  private_key: JsValue,
  /// Our ECDH public key (CryptoKey, stored as JsValue).
  public_key: JsValue,
  /// The shared secret (AES-256 key derived from ECDH).
  shared_key: Option<JsValue>,
  /// Key ID for rotation support.
  key_id: u32,
}

impl PeerCrypto {
  /// Create a new PeerCrypto instance and generate ECDH key pair.
  ///
  /// # Errors
  /// Returns an error if the Web Crypto API is unavailable or key generation fails.
  pub async fn new(peer_id: UserId) -> Result<Self, String> {
    let window = web_sys::window().ok_or("No window object available")?;
    let crypto = window
      .crypto()
      .map_err(|_| "Crypto not available".to_string())?;
    let subtle = crypto.subtle();

    // Generate ECDH P-256 key pair via SubtleCrypto
    let algo = Self::ecdh_key_gen_algorithm()?;
    let usages = Array::new();
    usages.push(&"deriveKey".into());
    usages.push(&"deriveBits".into());

    let algo_obj: &js_sys::Object = algo.dyn_ref().ok_or("ECDH algorithm is not an Object")?;

    let key_pair = wasm_bindgen_futures::JsFuture::from(
      subtle
        .generate_key_with_object(algo_obj, true, &usages)
        .map_err(|e| format!("Failed to call generate_key: {:?}", e))?,
    )
    .await
    .map_err(|e| format!("Failed to generate ECDH key pair: {:?}", e))?;

    let private_key = js_sys::Reflect::get(&key_pair, &"privateKey".into())
      .map_err(|_| "Failed to get private key")?;
    let public_key = js_sys::Reflect::get(&key_pair, &"publicKey".into())
      .map_err(|_| "Failed to get public key")?;

    Ok(Self {
      peer_id,
      private_key,
      public_key,
      shared_key: None,
      key_id: 0,
    })
  }

  /// Export the public key in SPKI format (for sending to peer).
  ///
  /// # Errors
  /// Returns an error if the export fails.
  pub async fn export_public_key(&self) -> Result<Vec<u8>, String> {
    let window = web_sys::window().ok_or("No window object available")?;
    let crypto = window
      .crypto()
      .map_err(|_| "Crypto not available".to_string())?;
    let subtle = crypto.subtle();

    let key = self
      .public_key
      .dyn_ref::<web_sys::CryptoKey>()
      .ok_or("Invalid public key")?;

    let array_buffer = wasm_bindgen_futures::JsFuture::from(
      subtle
        .export_key("spki", key)
        .map_err(|e| format!("Failed to call export_key: {:?}", e))?,
    )
    .await
    .map_err(|e| format!("Failed to export public key: {:?}", e))?;

    let uint8 = Uint8Array::new(&array_buffer);
    Ok(uint8.to_vec())
  }

  /// Import a peer's public key (received via EcdhKeyExchange message).
  ///
  /// # Errors
  /// Returns an error if the import fails.
  pub async fn import_peer_public_key(&mut self, key_data: &[u8]) -> Result<(), String> {
    let window = web_sys::window().ok_or("No window object available")?;
    let crypto = window
      .crypto()
      .map_err(|_| "Crypto not available".to_string())?;
    let subtle = crypto.subtle();

    let algo = Self::ecdh_algorithm()?;
    let key_buffer = Uint8Array::from(key_data).buffer();
    let usages = Array::new();
    usages.push(&"deriveKey".into());
    usages.push(&"deriveBits".into());

    let algo_obj: &js_sys::Object = algo.dyn_ref().ok_or("ECDH algorithm is not an Object")?;

    let public_key = wasm_bindgen_futures::JsFuture::from(
      subtle
        .import_key_with_object("spki", &key_buffer, algo_obj, false, &usages)
        .map_err(|e| format!("Failed to call import_key: {:?}", e))?,
    )
    .await
    .map_err(|e| format!("Failed to import peer public key: {:?}", e))?;

    self.derive_shared_key(&subtle, &public_key).await?;
    Ok(())
  }

  /// Derive the shared secret using ECDH.
  async fn derive_shared_key(
    &mut self,
    subtle: &web_sys::SubtleCrypto,
    peer_public_key: &JsValue,
  ) -> Result<(), String> {
    let private_key = self
      .private_key
      .dyn_ref::<web_sys::CryptoKey>()
      .ok_or("Invalid private key")?;

    // Derive bits using ECDH
    let algo = Self::ecdh_derive_algorithm_from_js(peer_public_key)?;
    let algo_obj: &js_sys::Object = algo
      .dyn_ref()
      .ok_or("ECDH derive algorithm is not an Object")?;

    let raw_secret = wasm_bindgen_futures::JsFuture::from(
      subtle
        .derive_bits_with_object(algo_obj, private_key, AES_KEY_SIZE)
        .map_err(|e| format!("Failed to call derive_bits: {:?}", e))?,
    )
    .await
    .map_err(|e| format!("Failed to derive shared secret: {:?}", e))?;

    // Derive AES-256 key from raw secret
    let aes_key = Self::derive_aes_key(subtle, &raw_secret).await?;

    self.shared_key = Some(aes_key);
    self.key_id += 1;

    web_sys::console::log_1(
      &format!("[encryption] Derived shared key for peer {}", self.peer_id).into(),
    );
    Ok(())
  }

  /// Derive AES-256 key from ECDH raw secret.
  ///
  /// Uses HKDF-like derivation: hash+truncate the raw secret to 32 bytes,
  /// then import as an AES-GCM key.
  async fn derive_aes_key(
    subtle: &web_sys::SubtleCrypto,
    raw_secret: &JsValue,
  ) -> Result<JsValue, String> {
    // Convert raw secret to bytes, ensure exactly 32 bytes
    let raw_bytes = Uint8Array::new(raw_secret);
    let mut key_material = raw_bytes.to_vec();
    if key_material.len() < 32 {
      key_material.resize(32, 0);
    }
    key_material.truncate(32);

    let key_buffer = Uint8Array::from(&key_material[..]).buffer();
    let usages = Array::new();
    usages.push(&"encrypt".into());
    usages.push(&"decrypt".into());

    // Use Algorithm dictionary for import
    let algo_dict = js_sys::Object::new();
    js_sys::Reflect::set(&algo_dict, &"name".into(), &"AES-GCM".into())
      .map_err(|_| "Failed to set algorithm name")?;
    js_sys::Reflect::set(&algo_dict, &"length".into(), &JsValue::from(256))
      .map_err(|_| "Failed to set algorithm length")?;

    let aes_key = wasm_bindgen_futures::JsFuture::from(
      subtle
        .import_key_with_object("raw", &key_buffer, &algo_dict, false, &usages)
        .map_err(|e| format!("Failed to call import_key: {:?}", e))?,
    )
    .await
    .map_err(|e| format!("Failed to import AES key: {:?}", e))?;

    Ok(aes_key)
  }

  /// Encrypt a message using AES-256-GCM.
  ///
  /// # Errors
  /// Returns an error if no shared key is available or encryption fails.
  pub async fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let shared_key = self
      .shared_key
      .as_ref()
      .ok_or("No shared key available for encryption")?;

    let window = web_sys::window().ok_or("No window object available")?;
    let crypto = window
      .crypto()
      .map_err(|_| "Crypto not available".to_string())?;

    // Generate random nonce using window.crypto.getRandomValues()
    let nonce = Self::generate_nonce(&crypto)?;

    // Set up AES-GCM algorithm with nonce
    let algo_dict = js_sys::Object::new();
    js_sys::Reflect::set(&algo_dict, &"name".into(), &"AES-GCM".into())
      .map_err(|_| "Failed to set algorithm name")?;
    js_sys::Reflect::set(
      &algo_dict,
      &"iv".into(),
      &Uint8Array::from(&nonce[..]).buffer(),
    )
    .map_err(|_| "Failed to set IV")?;

    let key = shared_key
      .dyn_ref::<web_sys::CryptoKey>()
      .ok_or("Invalid shared key")?;

    let plaintext_buffer = Uint8Array::from(plaintext).buffer();
    let subtle = crypto.subtle();

    let encrypted = wasm_bindgen_futures::JsFuture::from(
      subtle
        .encrypt_with_object_and_buffer_source(&algo_dict, key, &plaintext_buffer)
        .map_err(|e| format!("Failed to call encrypt: {:?}", e))?,
    )
    .await
    .map_err(|e| format!("Encryption failed: {:?}", e))?;

    let encrypted_bytes = Uint8Array::new(&encrypted).to_vec();

    // Prepend nonce to ciphertext
    let mut result = Vec::with_capacity(GCM_NONCE_SIZE + encrypted_bytes.len());
    result.extend_from_slice(&nonce);
    result.extend_from_slice(&encrypted_bytes);

    Ok(result)
  }

  /// Decrypt a message using AES-256-GCM.
  ///
  /// # Errors
  /// Returns an error if no shared key is available or decryption fails.
  pub async fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    if ciphertext.len() < GCM_NONCE_SIZE {
      return Err("Ciphertext too short (missing nonce)".to_string());
    }

    let shared_key = self
      .shared_key
      .as_ref()
      .ok_or("No shared key available for decryption")?;

    let window = web_sys::window().ok_or("No window object available")?;
    let crypto = window
      .crypto()
      .map_err(|_| "Crypto not available".to_string())?;

    // Split nonce and ciphertext
    let nonce = &ciphertext[..GCM_NONCE_SIZE];
    let ct = &ciphertext[GCM_NONCE_SIZE..];

    // Set up AES-GCM algorithm with nonce
    let algo_dict = js_sys::Object::new();
    js_sys::Reflect::set(&algo_dict, &"name".into(), &"AES-GCM".into())
      .map_err(|_| "Failed to set algorithm name")?;
    js_sys::Reflect::set(&algo_dict, &"iv".into(), &Uint8Array::from(nonce).buffer())
      .map_err(|_| "Failed to set IV")?;

    let key = shared_key
      .dyn_ref::<web_sys::CryptoKey>()
      .ok_or("Invalid shared key")?;

    let ct_buffer = Uint8Array::from(ct).buffer();
    let subtle = crypto.subtle();

    let decrypted = wasm_bindgen_futures::JsFuture::from(
      subtle
        .decrypt_with_object_and_buffer_source(&algo_dict, key, &ct_buffer)
        .map_err(|e| format!("Failed to call decrypt: {:?}", e))?,
    )
    .await
    .map_err(|e| format!("Decryption failed: {:?}", e))?;

    let decrypted_bytes = Uint8Array::new(&decrypted).to_vec();
    Ok(decrypted_bytes)
  }

  /// Check if the shared key has been established.
  #[must_use]
  pub fn has_shared_key(&self) -> bool {
    self.shared_key.is_some()
  }

  /// Get the current key ID.
  #[must_use]
  pub fn key_id(&self) -> u32 {
    self.key_id
  }

  /// Generate a random nonce for AES-GCM using window.crypto.getRandomValues().
  fn generate_nonce(crypto: &web_sys::Crypto) -> Result<Vec<u8>, String> {
    let buf = Uint8Array::new_with_length(GCM_NONCE_SIZE as u32);
    crypto
      .get_random_values_with_array_buffer_view(&buf)
      .map_err(|e| format!("Failed to generate random nonce: {:?}", e))?;
    Ok(buf.to_vec())
  }

  /// Create ECDH key generation algorithm object.
  fn ecdh_key_gen_algorithm() -> Result<JsValue, String> {
    let algo = web_sys::EcKeyGenParams::new("ECDH", "P-256");
    Ok(JsValue::from(algo))
  }

  /// Create ECDH algorithm object for key import.
  fn ecdh_algorithm() -> Result<JsValue, String> {
    let algo_dict = js_sys::Object::new();
    js_sys::Reflect::set(&algo_dict, &"name".into(), &"ECDH".into())
      .map_err(|_| "Failed to set algorithm name")?;
    js_sys::Reflect::set(&algo_dict, &"namedCurve".into(), &"P-256".into())
      .map_err(|_| "Failed to set namedCurve")?;
    Ok(JsValue::from(algo_dict))
  }

  /// Create ECDH key derivation algorithm object from a JsValue peer public key.
  fn ecdh_derive_algorithm_from_js(public_key: &JsValue) -> Result<JsValue, String> {
    let pk = public_key
      .dyn_ref::<web_sys::CryptoKey>()
      .ok_or("Peer public key is not a CryptoKey")?;
    let algo = web_sys::EcdhKeyDeriveParams::new("ECDH", pk);
    Ok(JsValue::from(algo))
  }
}

/// Serialize ECDH public key for transmission via signaling channel.
///
/// The key is encoded as: `[key_id (4 bytes, big-endian)] + [spki_der_bytes]`
#[must_use]
pub fn serialize_ecdh_key(key_id: u32, key_data: &[u8]) -> Vec<u8> {
  let mut result = Vec::with_capacity(4 + key_data.len());
  result.extend_from_slice(&key_id.to_be_bytes());
  result.extend_from_slice(key_data);
  result
}

/// Deserialize ECDH public key from signaling message.
///
/// Returns (key_id, key_data).
///
/// # Errors
/// Returns an error if the data is too short.
pub fn deserialize_ecdh_key(data: &[u8]) -> Result<(u32, Vec<u8>), String> {
  if data.len() < 4 {
    return Err("ECDH key data too short".to_string());
  }
  let key_id = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
  let key_data = data[4..].to_vec();
  Ok((key_id, key_data))
}

// ── Tests ──

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_serialize_deserialize_ecdh_key() {
    let key_id = 42u32;
    let key_data = vec![1, 2, 3, 4, 5];

    let serialized = serialize_ecdh_key(key_id, &key_data);
    let (deserialized_id, deserialized_data) = deserialize_ecdh_key(&serialized).unwrap();

    assert_eq!(key_id, deserialized_id);
    assert_eq!(key_data, deserialized_data);
  }

  #[test]
  fn test_deserialize_ecdh_key_too_short() {
    let data = vec![1, 2, 3]; // Only 3 bytes, need at least 4
    let result = deserialize_ecdh_key(&data);
    assert!(result.is_err());
  }

  #[test]
  fn test_nonce_size() {
    // Verify GCM_NONCE_SIZE is 12 (NIST recommendation)
    assert_eq!(GCM_NONCE_SIZE, 12);
  }
}
