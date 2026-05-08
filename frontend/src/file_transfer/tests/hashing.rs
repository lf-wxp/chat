use super::hash;

#[test]
fn native_sha256_is_deterministic() {
  let h1 = futures::executor::block_on(hash::sha256(b"alice sends file"))
    .expect("native sha256 should return Ok on tests");
  let h2 = futures::executor::block_on(hash::sha256(b"alice sends file"))
    .expect("native sha256 should return Ok on tests");
  assert_eq!(h1, h2);
  let h3 = futures::executor::block_on(hash::sha256(b"different input"))
    .expect("native sha256 should return Ok on tests");
  assert_ne!(h1, h3);
}

/// Verify the native SHA-256 produces the well-known digest for the
/// empty string (proves we're using real SHA-256, not a pseudo-hash).
#[test]
fn native_sha256_matches_known_vector() {
  let digest = futures::executor::block_on(hash::sha256(b""))
    .expect("native sha256 should return Ok for empty input");
  // SHA-256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
  let expected = [
    0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9, 0x24,
    0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52, 0xb8, 0x55,
  ];
  assert_eq!(digest, expected, "SHA-256 empty-string vector mismatch");
}

/// `hex()` formatter produces the correct lowercase hex string.
#[test]
fn hex_formatter_produces_correct_output() {
  let digest = [
    0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9, 0x24,
    0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52, 0xb8, 0x55,
  ];
  assert_eq!(
    hash::hex(&digest),
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  );
}
