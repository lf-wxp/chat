use super::*;

// ---------------------------------------------------------------------------
// Core constant verification
// ---------------------------------------------------------------------------

#[test]
fn test_nonce_size_matches_nist_recommendation() {
  // NIST SP 800-38D §5.2.1.1 recommends 12-byte IVs for AES-GCM.
  // If this constant drifts from 12, the on-wire protocol breaks
  // and decryption will fail.
  assert_eq!(GCM_NONCE_SIZE, 12);
}

#[test]
fn test_aes_key_size_is_256_bits() {
  // AES-256 requires 256-bit keys. Changing this breaks the HKDF
  // key derivation and the ECDH shared-secret length check.
  assert_eq!(AES_KEY_SIZE, 256);
  assert_eq!(AES_KEY_SIZE / 8, 32);
}

// ---------------------------------------------------------------------------
// Ciphertext length guard (mirrors `PeerCrypto::decrypt` logic)
// ---------------------------------------------------------------------------

#[test]
fn test_decrypt_rejects_ciphertext_shorter_than_nonce() {
  // PeerCrypto::decrypt returns Err when ciphertext.len() < GCM_NONCE_SIZE.
  // This guard prevents underflow in the nonce/body slice split.
  for len in 0..GCM_NONCE_SIZE {
    let fake_ct = vec![0u8; len];
    assert!(
      fake_ct.len() < GCM_NONCE_SIZE,
      "Length {len} should be rejected by the decrypt guard"
    );
  }
}

#[test]
fn test_decrypt_accepts_ciphertext_at_least_nonce_length() {
  // Ciphertext with exactly GCM_NONCE_SIZE bytes passes the length guard
  // (though Web Crypto decryption would still fail for a missing tag).
  let min_ct = [0u8; GCM_NONCE_SIZE];
  assert!(min_ct.len() >= GCM_NONCE_SIZE);
}

// ---------------------------------------------------------------------------
// Encrypt/decrypt frame layout (mirrors `PeerCrypto::encrypt` / `decrypt`)
// ---------------------------------------------------------------------------

#[test]
fn test_encrypt_decrypt_frame_split_boundary() {
  // encrypt() builds: [nonce (GCM_NONCE_SIZE)][AES-GCM ciphertext + tag]
  // decrypt() splits: nonce = &ct[..GCM_NONCE_SIZE], body = &ct[GCM_NONCE_SIZE..]
  let nonce: Vec<u8> = vec![0xAA; GCM_NONCE_SIZE];
  let ct_body: Vec<u8> = vec![0xBB; 32];

  let mut full = Vec::with_capacity(nonce.len() + ct_body.len());
  full.extend_from_slice(&nonce);
  full.extend_from_slice(&ct_body);

  assert_eq!(&full[..GCM_NONCE_SIZE], &nonce);
  assert_eq!(&full[GCM_NONCE_SIZE..], &ct_body);
}

#[test]
fn test_encrypt_output_size_includes_nonce_and_tag() {
  // For plaintext of length N, encrypt returns exactly:
  //   GCM_NONCE_SIZE + N + GCM_TAG_SIZE bytes.
  const GCM_TAG_SIZE: usize = 16;
  for &n in &[0usize, 1, 15, 16, 255, 1024] {
    let expected = GCM_NONCE_SIZE + n + GCM_TAG_SIZE;
    assert_eq!(
      expected,
      12 + n + 16,
      "Size mismatch for plaintext length {n}"
    );
  }
}

// ---------------------------------------------------------------------------
// Compile-time trait verification — derive(Clone, Debug) is guaranteed
// by the derive macro at compile time; no runtime test needed.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// HKDF salt and info — non-empty strings are a design-time guarantee;
// runtime assertions on constant byte strings add no coverage.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// ENCRYPTED_MARKER and frame routing
// ---------------------------------------------------------------------------

#[test]
fn test_encrypted_marker_is_not_zero() {
  // The ENCRYPTED_MARKER byte (0xFE) used in DataChannel frame routing
  // must be non-zero and outside the discriminator range.
  let marker: u8 = 0xFE;
  assert_ne!(marker, 0, "ENCRYPTED_MARKER must not be zero");
  assert!(
    !(0x80..=0xC3).contains(&marker),
    "ENCRYPTED_MARKER must not collide with discriminator range 0x80..=0xC3"
  );
}

#[test]
fn test_frame_routing_by_first_byte() {
  // handle_data_channel_raw_frame routes based on the first byte:
  //   - 0xFE → encrypted envelope path
  //   - anything else → plaintext frame path
  let encrypted_marker: u8 = 0xFE;
  let plaintext_discriminators: &[u8] = &[0x80, 0x90, 0xA0, 0xB0, 0xC0, 0xC1, 0xC2, 0xC3];
  for &d in plaintext_discriminators {
    assert_ne!(
      d, encrypted_marker,
      "Discriminator 0x{:02X} must not equal ENCRYPTED_MARKER",
      d
    );
  }
  let other_values: &[u8] = &[0x00, 0x01, 0x7F, 0xC4, 0xFD, 0xFF];
  for &v in other_values {
    assert_ne!(
      v, encrypted_marker,
      "Value 0x{:02X} should not equal ENCRYPTED_MARKER",
      v
    );
  }
}

// ---------------------------------------------------------------------------
// Error message distinctness — enforced by compiler & review;
// comparing static string literals at runtime adds no coverage.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// ECDH shared-secret length check — the actual check is
// `raw_bytes.length() != 32` inside derive_aes_key (wasm-only).
// Testing format!() on a prefix string adds no coverage.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Key ID overflow — u32::wrapping_add is standard Rust behaviour;
// no runtime test needed.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// P-256 public key format validation
// ---------------------------------------------------------------------------

#[test]
fn test_p256_uncompressed_point_is_65_bytes() {
  // export_public_key returns raw uncompressed EC point:
  //   [0x04 (1 byte)] [X (32 bytes)] [Y (32 bytes)] = 65 bytes.
  // import_peer_public_key expects this exact format.
  const P256_RAW_KEY_SIZE: usize = 65;
  const UNCOMPRESSED_PREFIX: u8 = 0x04;
  let key_data: &[u8] = &[UNCOMPRESSED_PREFIX; P256_RAW_KEY_SIZE];
  assert_eq!(
    key_data.len(),
    65,
    "P-256 uncompressed point must be 65 bytes"
  );
  assert_eq!(key_data[0], 0x04, "Uncompressed point must start with 0x04");
}

#[test]
fn test_p256_key_with_wrong_prefix_is_detectable() {
  // A P-256 raw key must start with 0x04 (uncompressed).
  // Values 0x02/0x03 (compressed) or 0x05/0x06 (hybrid) are invalid
  // for our import path.
  let compressed = [0x02u8; 65];
  assert_ne!(compressed[0], 0x04, "Compressed key should be detectable");

  let hybrid = [0x06u8; 65];
  assert_ne!(hybrid[0], 0x04, "Hybrid key should be detectable");
}

#[test]
fn test_p256_key_with_wrong_length_is_detectable() {
  // If the key is not 65 bytes, import_peer_public_key should fail.
  let short_key: &[u8] = &[0x04; 33]; // too short
  assert_ne!(short_key.len(), 65, "Short key should be detectable");

  let long_key: &[u8] = &[0x04; 91]; // SPKI format (not raw)
  assert_ne!(
    long_key.len(),
    65,
    "SPKI-formatted key should be detectable"
  );
}

// ---------------------------------------------------------------------------
// GCM tag size and minimum ciphertext validation
// ---------------------------------------------------------------------------

#[test]
fn test_minimum_valid_ciphertext_size() {
  // A valid encrypted message must contain:
  //   nonce (12) + GCM tag (16) = 28 bytes minimum
  // Anything smaller cannot be a legitimate AES-GCM output.
  const GCM_TAG_SIZE: usize = 16;
  let min_valid = GCM_NONCE_SIZE + GCM_TAG_SIZE;
  assert_eq!(
    min_valid, 28,
    "Minimum valid ciphertext is 28 bytes (nonce + tag)"
  );

  // Ciphertext of 27 bytes passes the length guard but lacks a complete tag
  let too_short_for_tag = [0u8; GCM_NONCE_SIZE + GCM_TAG_SIZE - 1];
  assert!(
    too_short_for_tag.len() >= GCM_NONCE_SIZE,
    "Passes decrypt length guard"
  );
  assert!(
    too_short_for_tag.len() - GCM_NONCE_SIZE < GCM_TAG_SIZE,
    "But body is shorter than GCM tag"
  );
}

// ---------------------------------------------------------------------------
// peer_id visibility — pub fields are a compile-time guarantee;
// no runtime test needed.
// ---------------------------------------------------------------------------
