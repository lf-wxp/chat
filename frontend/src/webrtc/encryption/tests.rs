use super::*;

#[test]
fn test_nonce_size() {
  // Verify GCM_NONCE_SIZE is 12 (NIST recommendation)
  assert_eq!(GCM_NONCE_SIZE, 12);
}
