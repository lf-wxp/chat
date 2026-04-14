//! Malformed message handling tests.

use super::*;

#[test]
fn test_decode_frame_too_short() {
  // Empty buffer
  let result = decode_frame(&[]);
  assert!(result.is_err());

  // Only 1 byte (need at least 3)
  let result = decode_frame(&[0xBC]);
  assert!(result.is_err());

  // Only 2 bytes (need at least 3)
  let result = decode_frame(&[0xBC, 0xBC]);
  assert!(result.is_err());
}

#[test]
fn test_decode_frame_invalid_magic_number() {
  // Wrong magic number
  let bytes = [0xAB, 0xCD, 0x80, 1, 2, 3, 4, 5];
  let result = decode_frame(&bytes);
  assert!(result.is_err());

  // Partially correct magic number (first byte only)
  let bytes = [0xBC, 0xAB, 0x80, 1, 2, 3, 4, 5];
  let result = decode_frame(&bytes);
  assert!(result.is_err());

  // Partially correct magic number (second byte only)
  let bytes = [0xAB, 0xBC, 0x80, 1, 2, 3, 4, 5];
  let result = decode_frame(&bytes);
  assert!(result.is_err());
}

#[test]
fn test_decode_frame_empty_payload() {
  // Valid magic number + message type but no payload
  let bytes = [0xBC, 0xBC, 0x80];
  let result = decode_frame(&bytes);
  assert!(result.is_err());
}

#[test]
fn test_decode_signaling_message_invalid_type() {
  // Valid frame with unknown message type discriminator
  let unknown_type = 0xFF; // Unknown discriminator
  let frame = MessageFrame::new(unknown_type, vec![1, 2, 3, 4, 5]);
  let result = decode_signaling_message(&frame);
  assert!(result.is_err());
}

#[test]
fn test_decode_signaling_message_corrupted_payload() {
  // Valid message type but corrupted bitcode payload
  let frame = MessageFrame::new(0x00, vec![0xDE, 0xAD, 0xBE, 0xEF]);
  let result = decode_signaling_message(&frame);
  assert!(result.is_err());
}

#[test]
fn test_encode_frame_empty_payload_error() {
  let frame = MessageFrame::new(0x80, vec![]);
  let result = encode_frame(&frame);
  assert!(result.is_err());
}

#[test]
fn test_decode_frame_truncated_message() {
  // Valid header but truncated payload
  let bytes = [0xBC, 0xBC, 0x80, 1];
  let result = decode_frame(&bytes);
  // decode_frame only checks magic number and that payload is non-empty
  // This should succeed with a 1-byte payload
  assert!(result.is_ok());
  let frame = result.unwrap();
  assert_eq!(frame.payload, vec![1]);
}

#[test]
fn test_chunked_message_decode_corrupted() {
  // Random bytes that cannot be decoded as ChunkedMessage
  let corrupted_bytes = [0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE];
  let result = ChunkedMessage::decode(&corrupted_bytes);
  assert!(result.is_err());
}
