use super::*;

#[test]
fn base64_matches_reference_vectors() {
  assert_eq!(base64_encode(b""), "");
  assert_eq!(base64_encode(b"f"), "Zg==");
  assert_eq!(base64_encode(b"fo"), "Zm8=");
  assert_eq!(base64_encode(b"foo"), "Zm9v");
  assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
  assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
  assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
}

#[test]
fn data_url_preserves_mime_and_payload() {
  let url = bytes_to_data_url("image/png", &[1, 2, 3]);
  assert!(url.starts_with("data:image/png;base64,"));
  assert!(url.ends_with("AQID"));
}

#[test]
fn nanos_to_ms_rounds_down() {
  assert_eq!(nanos_to_ms(1_500_000), 1);
  assert_eq!(nanos_to_ms(0), 0);
  assert_eq!(nanos_to_ms(1_999_999), 1);
}
