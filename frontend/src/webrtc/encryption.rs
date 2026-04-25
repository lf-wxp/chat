//! End-to-end encryption for WebRTC DataChannel messages.
//!
//! Uses ECDH P-256 for key exchange and AES-256-GCM for encryption.
//! All cryptographic operations are performed using the Web Crypto API via
//! `web-sys` and `js-sys` bindings.

use js_sys::{Array, Uint8Array};
use message::UserId;
use wasm_bindgen::prelude::*;

/// Key size for AES-256 (32 bytes).
pub const AES_KEY_SIZE: u32 = 256;

/// AES-GCM nonce size (12 bytes).
pub const GCM_NONCE_SIZE: usize = 12;

/// Type-safe wrapper around a Web Crypto `CryptoKey` value (P1-7).
///
/// The Web Crypto API returns `CryptoKey` instances as opaque `JsValue`
/// handles; the raw key material is not extractable from JS and is never
/// exposed to Rust. Historically `PeerCrypto` stored these as bare
/// `JsValue`s, which meant:
///
/// * every call site had to re-run `dyn_ref::<CryptoKey>()` and handle the
///   `None` branch, duplicating error messages;
/// * nothing at the type level prevented passing an arbitrary `JsValue`
///   (e.g. a plain JS object from `Reflect::get`) where a `CryptoKey` was
///   expected — the mistake would only surface at runtime as an opaque
///   DOMException from Web Crypto.
///
/// `CryptoKeyValue` closes both gaps: its single constructor validates the
/// underlying JS value once, and downstream accessors return a typed
/// `&web_sys::CryptoKey` reference with no further checks.
///
/// The type is intentionally `pub(crate)` — callers outside the `webrtc`
/// module have no reason to hold raw crypto key handles.
#[derive(Debug, Clone)]
pub(crate) struct CryptoKeyValue(JsValue);

impl CryptoKeyValue {
  /// Validate a `JsValue` is actually a Web Crypto `CryptoKey` and wrap it.
  ///
  /// # Errors
  /// Returns an error string if `value` is not a `CryptoKey` instance.
  fn from_js(value: JsValue) -> Result<Self, String> {
    if value.dyn_ref::<web_sys::CryptoKey>().is_none() {
      return Err("Expected a CryptoKey value".to_string());
    }
    Ok(Self(value))
  }

  /// Borrow the underlying `CryptoKey`.
  ///
  /// Safe because the only constructor (`from_js`) verifies the dynamic
  /// type, so `unchecked_ref` never observes a non-`CryptoKey` value.
  fn as_crypto_key(&self) -> &web_sys::CryptoKey {
    self.0.unchecked_ref()
  }
}

/// Manages ECDH key exchange and AES-256-GCM encryption for a peer.
#[derive(Debug, Clone)]
pub struct PeerCrypto {
  /// The peer's user ID.
  pub peer_id: UserId,
  /// Our ECDH private key (validated `CryptoKey` wrapper).
  private_key: CryptoKeyValue,
  /// Our ECDH public key (validated `CryptoKey` wrapper).
  public_key: CryptoKeyValue,
  /// The shared AES-256-GCM key derived from ECDH, or `None` until the
  /// peer's public key has been imported.
  shared_key: Option<CryptoKeyValue>,
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
      private_key: CryptoKeyValue::from_js(private_key)?,
      public_key: CryptoKeyValue::from_js(public_key)?,
      shared_key: None,
      key_id: 0,
    })
  }

  /// Export the public key in raw format (uncompressed EC point, 65 bytes for P-256).
  ///
  /// Uses the "raw" format instead of "spki" to keep the key compact
  /// (65 bytes vs ~91 bytes for SPKI). The raw format contains just the
  /// uncompressed point: `[0x04, X(32 bytes), Y(32 bytes)]`.
  ///
  /// # Errors
  /// Returns an error if the export fails.
  pub async fn export_public_key(&self) -> Result<Vec<u8>, String> {
    let window = web_sys::window().ok_or("No window object available")?;
    let crypto = window
      .crypto()
      .map_err(|_| "Crypto not available".to_string())?;
    let subtle = crypto.subtle();

    let array_buffer = wasm_bindgen_futures::JsFuture::from(
      subtle
        .export_key("raw", self.public_key.as_crypto_key())
        .map_err(|e| format!("Failed to call export_key: {:?}", e))?,
    )
    .await
    .map_err(|e| format!("Failed to export public key: {:?}", e))?;

    let uint8 = Uint8Array::new(&array_buffer);
    Ok(uint8.to_vec())
  }

  /// Import a peer's public key (received via EcdhKeyExchange message).
  ///
  /// Expects the key in raw format (uncompressed EC point, 65 bytes for P-256).
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
    // Web Crypto spec requires ECDH *public* keys to be imported with an
    // empty `keyUsages` array — only the matching private key carries the
    // `deriveKey`/`deriveBits` usages. Passing any non-empty list here
    // triggers `SyntaxError: Cannot create a key using the specified key
    // usages.` in every browser implementation.
    let usages = Array::new();

    let algo_obj: &js_sys::Object = algo.dyn_ref().ok_or("ECDH algorithm is not an Object")?;

    let public_key = wasm_bindgen_futures::JsFuture::from(
      subtle
        .import_key_with_object("raw", &key_buffer, algo_obj, false, &usages)
        .map_err(|e| format!("Failed to call import_key: {:?}", e))?,
    )
    .await
    .map_err(|e| format!("Failed to import peer public key: {:?}", e))?;

    let peer_public = CryptoKeyValue::from_js(public_key)?;
    self.derive_shared_key(&subtle, &peer_public).await?;
    Ok(())
  }

  /// Derive the shared secret using ECDH.
  async fn derive_shared_key(
    &mut self,
    subtle: &web_sys::SubtleCrypto,
    peer_public_key: &CryptoKeyValue,
  ) -> Result<(), String> {
    // Derive bits using ECDH
    let algo = Self::ecdh_derive_algorithm_from_key(peer_public_key)?;
    let algo_obj: &js_sys::Object = algo
      .dyn_ref()
      .ok_or("ECDH derive algorithm is not an Object")?;

    let raw_secret = wasm_bindgen_futures::JsFuture::from(
      subtle
        .derive_bits_with_object(algo_obj, self.private_key.as_crypto_key(), AES_KEY_SIZE)
        .map_err(|e| format!("Failed to call derive_bits: {:?}", e))?,
    )
    .await
    .map_err(|e| format!("Failed to derive shared secret: {:?}", e))?;

    // Verify ECDH P-256 shared secret length before HKDF (P1-15 fix).
    // All spec-compliant browsers return 32 bytes (256 bits) for P-256.
    let raw_bytes = Uint8Array::new(&raw_secret);
    if raw_bytes.length() != 32 {
      return Err(format!(
        "ECDH shared secret must be 32 bytes for P-256, got {}",
        raw_bytes.length()
      ));
    }

    // Derive AES-256 key from raw secret
    let aes_key = Self::derive_aes_key(subtle, &raw_secret).await?;

    self.shared_key = Some(CryptoKeyValue::from_js(aes_key)?);
    self.key_id += 1;

    web_sys::console::log_1(
      &format!("[encryption] Derived shared key for peer {}", self.peer_id).into(),
    );
    Ok(())
  }

  /// Derive AES-256 key from ECDH raw secret using HKDF-SHA256.
  ///
  /// Uses the Web Crypto API's `deriveKey` with HKDF to properly derive
  /// an AES-256-GCM key from the ECDH shared secret. This follows
  /// cryptographic best practices (RFC 5869) for key separation.
  async fn derive_aes_key(
    subtle: &web_sys::SubtleCrypto,
    raw_secret: &JsValue,
  ) -> Result<JsValue, String> {
    // Import the raw ECDH secret as key material for HKDF
    let raw_bytes = Uint8Array::new(raw_secret);
    let key_material_buffer = raw_bytes.buffer();

    let import_algo = js_sys::Object::new();
    js_sys::Reflect::set(&import_algo, &"name".into(), &"HKDF".into())
      .map_err(|_| "Failed to set algorithm name")?;

    let import_usages = Array::new();
    import_usages.push(&"deriveKey".into());

    let hkdf_key = wasm_bindgen_futures::JsFuture::from(
      subtle
        .import_key_with_object(
          "raw",
          &key_material_buffer,
          &import_algo,
          false,
          &import_usages,
        )
        .map_err(|e| format!("Failed to call import_key for HKDF: {:?}", e))?,
    )
    .await
    .map_err(|e| format!("Failed to import HKDF key material: {:?}", e))?;

    // Set up HKDF parameters: SHA-256 with salt and info
    let hkdf_algo = js_sys::Object::new();
    js_sys::Reflect::set(&hkdf_algo, &"name".into(), &"HKDF".into())
      .map_err(|_| "Failed to set HKDF name")?;
    js_sys::Reflect::set(&hkdf_algo, &"hash".into(), &"SHA-256".into())
      .map_err(|_| "Failed to set HKDF hash")?;

    // Salt: fixed application-specific value for E2EE chat key derivation.
    // A fixed salt is acceptable here because the ECDH shared secret is
    // already unique per peer pair.
    let salt = Uint8Array::from(b"webrtc-e2ee-chat-hkdf-salt-v1".as_slice());
    js_sys::Reflect::set(&hkdf_algo, &"salt".into(), &salt.buffer())
      .map_err(|_| "Failed to set HKDF salt")?;

    // Info: application-specific context string for key separation
    let info = Uint8Array::from(b"AES-256-GCM".as_slice());
    js_sys::Reflect::set(&hkdf_algo, &"info".into(), &info.buffer())
      .map_err(|_| "Failed to set HKDF info")?;

    // Derive AES-256-GCM key
    let aes_algo = js_sys::Object::new();
    js_sys::Reflect::set(&aes_algo, &"name".into(), &"AES-GCM".into())
      .map_err(|_| "Failed to set AES algorithm name")?;
    js_sys::Reflect::set(&aes_algo, &"length".into(), &JsValue::from(256))
      .map_err(|_| "Failed to set AES algorithm length")?;

    let derive_usages = Array::new();
    derive_usages.push(&"encrypt".into());
    derive_usages.push(&"decrypt".into());

    let hkdf_crypto_key: web_sys::CryptoKey = hkdf_key
      .dyn_into()
      .map_err(|_| "HKDF key is not a CryptoKey")?;

    let aes_key = wasm_bindgen_futures::JsFuture::from(
      subtle
        .derive_key_with_object_and_object(
          &hkdf_algo,
          &hkdf_crypto_key,
          &aes_algo,
          false,
          &derive_usages,
        )
        .map_err(|e| format!("Failed to call deriveKey: {:?}", e))?,
    )
    .await
    .map_err(|e| format!("HKDF key derivation failed: {:?}", e))?;

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

    let plaintext_buffer = Uint8Array::from(plaintext).buffer();
    let subtle = crypto.subtle();

    let encrypted = wasm_bindgen_futures::JsFuture::from(
      subtle
        .encrypt_with_object_and_buffer_source(
          &algo_dict,
          shared_key.as_crypto_key(),
          &plaintext_buffer,
        )
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

    let ct_buffer = Uint8Array::from(ct).buffer();
    let subtle = crypto.subtle();

    let decrypted = wasm_bindgen_futures::JsFuture::from(
      subtle
        .decrypt_with_object_and_buffer_source(&algo_dict, shared_key.as_crypto_key(), &ct_buffer)
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

  /// Create ECDH key derivation algorithm object from a validated peer
  /// public key. The `CryptoKeyValue` wrapper guarantees the underlying
  /// value is a `CryptoKey`, so this helper no longer needs to perform
  /// its own `dyn_ref` check (P1-7).
  fn ecdh_derive_algorithm_from_key(public_key: &CryptoKeyValue) -> Result<JsValue, String> {
    let algo = web_sys::EcdhKeyDeriveParams::new("ECDH", public_key.as_crypto_key());
    Ok(JsValue::from(algo))
  }
}

// ── Tests ──

#[cfg(test)]
mod tests;

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests;
