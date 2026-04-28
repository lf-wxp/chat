//! SHA-256 hashing helpers backed by the Web Crypto API.
//!
//! The browser's `SubtleCrypto.digest("SHA-256", ...)` is hardware
//! accelerated in every target browser; we thin-wrap it here so the
//! rest of the file-transfer module can call `sha256(&bytes).await`
//! without touching `web_sys` directly.

#[cfg(target_arch = "wasm32")]
use js_sys::Uint8Array;

/// Compute the SHA-256 digest of `bytes`.
///
/// # Errors
/// Returns a human-readable error when the browser APIs are
/// unavailable or the digest call is rejected.
#[cfg(target_arch = "wasm32")]
pub async fn sha256(bytes: &[u8]) -> Result<[u8; 32], String> {
  let window = web_sys::window().ok_or_else(|| "No window object available".to_string())?;
  let crypto = window
    .crypto()
    .map_err(|_| "Crypto API not available".to_string())?;
  let subtle = crypto.subtle();

  let buffer = Uint8Array::new_with_length(
    u32::try_from(bytes.len()).map_err(|_| "Input too large".to_string())?,
  );
  buffer.copy_from(bytes);

  let promise = subtle
    .digest_with_str_and_buffer_source("SHA-256", &buffer)
    .map_err(|e| format!("digest() rejected: {e:?}"))?;

  let js_result = wasm_bindgen_futures::JsFuture::from(promise)
    .await
    .map_err(|e| format!("digest() failed: {e:?}"))?;

  let digest = Uint8Array::new(&js_result).to_vec();
  <[u8; 32]>::try_from(digest.as_slice())
    .map_err(|_| "Digest length mismatch (expected 32 bytes)".to_string())
}

/// Native build path using the `sha2` crate so unit tests verify
/// real SHA-256 correctness instead of a pseudo-hash fallback.
///
/// The `sha2` crate also compiles for `wasm32-unknown-unknown`, but
/// the WASM build uses the browser's hardware-accelerated
/// `SubtleCrypto` above.
#[cfg(not(target_arch = "wasm32"))]
pub async fn sha256(bytes: &[u8]) -> Result<[u8; 32], String> {
  use sha2::Digest;
  let digest = sha2::Sha256::digest(bytes);
  Ok(digest.into())
}

/// Synchronous SHA-256 helper for small slices (used in hot dispatch
/// loops where spawning an async task per chunk is prohibitive).
///
/// On WASM this falls back to a software implementation via the
/// `sha2` crate (compiled for wasm32); on native it uses the same
/// `sha2` path. The browser's SubtleCrypto is async-only and is
/// therefore reserved for the full-file hash in `start_outgoing_transfer`.
#[must_use]
pub fn sha256_sync(bytes: &[u8]) -> [u8; 32] {
  use sha2::Digest;
  let digest = sha2::Sha256::digest(bytes);
  digest.into()
}

/// Hex-format a 32-byte digest for display / debugging.
#[must_use]
pub fn hex(digest: &[u8; 32]) -> String {
  let mut s = String::with_capacity(64);
  for b in digest {
    s.push_str(&format!("{b:02x}"));
  }
  s
}
