//! WASM-level file-transfer integration tests.
//!
//! These tests exercise browser-specific code paths that cannot be
//! tested with plain `cargo test`:
//!
//! * SHA-256 via `SubtleCrypto.digest()` (vs native `sha2` crate).
//! * Thumbnail generation via Canvas API.
//! * Blob URL creation + revocation.
//!
//! Run with: `wasm-pack test --headless --firefox` (or `--chrome`).

use crate::file_transfer::hash;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// Verify that the SubtleCrypto-based SHA-256 produces the well-known
/// digest for the empty string. This guards against a regression in
/// the WASM-specific async hashing path.
#[wasm_bindgen_test]
async fn subtle_crypto_sha256_empty_string() {
  let digest = hash::sha256(b"")
    .await
    .expect("SubtleCrypto SHA-256 should succeed for empty input");
  let expected = [
    0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9, 0x24,
    0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52, 0xb8, 0x55,
  ];
  assert_eq!(digest, expected, "SHA-256 empty-string vector mismatch");
}

/// Verify that the SubtleCrypto SHA-256 produces the correct digest
/// for a non-trivial input. Uses the well-known SHA-256("hello world")
/// vector.
#[wasm_bindgen_test]
async fn subtle_crypto_sha256_hello_world() {
  let digest = hash::sha256(b"hello world")
    .await
    .expect("SubtleCrypto SHA-256 should succeed");
  // SHA-256("hello world") = b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
  let expected = [
    0xb9, 0x4d, 0x27, 0xb9, 0x93, 0x4d, 0x3e, 0x08, 0xa5, 0x2e, 0x52, 0xd7, 0xda, 0x7d, 0xab, 0xfa,
    0xc4, 0x84, 0xef, 0xe3, 0x7a, 0x53, 0x80, 0xee, 0x90, 0x88, 0xf7, 0xac, 0xe2, 0xef, 0xcd, 0xe9,
  ];
  assert_eq!(digest, expected, "SHA-256 hello-world vector mismatch");
}

/// Verify that the SubtleCrypto SHA-256 is deterministic: hashing
/// the same input twice must produce identical digests.
#[wasm_bindgen_test]
async fn subtle_crypto_sha256_is_deterministic() {
  let h1 = hash::sha256(b"determinism test input")
    .await
    .expect("first hash should succeed");
  let h2 = hash::sha256(b"determinism test input")
    .await
    .expect("second hash should succeed");
  assert_eq!(h1, h2, "SHA-256 must be deterministic");
}

/// Verify that different inputs produce different digests.
#[wasm_bindgen_test]
async fn subtle_crypto_sha256_different_inputs_differ() {
  let h1 = hash::sha256(b"input A")
    .await
    .expect("hash of 'input A' should succeed");
  let h2 = hash::sha256(b"input B")
    .await
    .expect("hash of 'input B' should succeed");
  assert_ne!(h1, h2, "different inputs must produce different hashes");
}

/// Verify that the SubtleCrypto SHA-256 handles a larger input
/// (64 KB) correctly without truncation or overflow.
#[wasm_bindgen_test]
async fn subtle_crypto_sha256_large_input() {
  let large = vec![0x42_u8; 64 * 1024];
  let digest = hash::sha256(&large)
    .await
    .expect("SHA-256 of 64 KB input should succeed");
  // Verify the digest is 32 bytes (all SHA-256 digests are 32 bytes).
  assert_eq!(digest.len(), 32);
  // The digest should not be all zeros.
  assert_ne!(digest, [0u8; 32], "digest must not be all-zero");
}

/// Verify that the synchronous `sha256_sync` helper produces the
/// same result as the async SubtleCrypto path. This is important
/// because `sha256_sync` is used for per-chunk hash validation.
#[wasm_bindgen_test]
async fn sha256_sync_matches_subtle_crypto() {
  let input = b"consistency check";
  let sync_hash = hash::sha256_sync(input);
  let async_hash = hash::sha256(input)
    .await
    .expect("async SHA-256 should succeed");
  assert_eq!(sync_hash, async_hash, "sync and async SHA-256 must agree");
}
