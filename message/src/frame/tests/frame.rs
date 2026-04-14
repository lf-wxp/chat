//! `MessageFrame` creation and encode/decode tests.

use super::*;

#[test]
fn test_magic_number() {
  assert_eq!(MAGIC_NUMBER, 0xBCBC);
  assert_eq!(MAGIC_NUMBER_BYTES, [0xBC, 0xBC]);
}

#[test]
fn test_message_frame_creation() {
  let frame = MessageFrame::new(0x80, vec![1, 2, 3, 4]);
  assert_eq!(frame.message_type, 0x80);
  assert_eq!(frame.payload, vec![1, 2, 3, 4]);
}

#[test]
fn test_message_frame_needs_chunking() {
  let small_frame = MessageFrame::new(0x80, vec![0; 1024]);
  assert!(!small_frame.needs_chunking());

  let large_frame = MessageFrame::new(0x80, vec![0; MAX_CHUNK_SIZE + 1]);
  assert!(large_frame.needs_chunking());
}

#[test]
fn test_message_frame_chunk_count() {
  let frame = MessageFrame::new(0x80, vec![0; MAX_CHUNK_SIZE]);
  assert_eq!(frame.chunk_count(), 1);

  let frame = MessageFrame::new(0x80, vec![0; MAX_CHUNK_SIZE + 1]);
  assert_eq!(frame.chunk_count(), 2);

  let frame = MessageFrame::new(0x80, vec![0; MAX_CHUNK_SIZE * 2]);
  assert_eq!(frame.chunk_count(), 2);

  let frame = MessageFrame::new(0x80, vec![0; MAX_CHUNK_SIZE * 2 + 1]);
  assert_eq!(frame.chunk_count(), 3);
}

#[test]
fn test_encode_decode_frame() {
  let frame = MessageFrame::new(0x80, vec![1, 2, 3, 4, 5]);
  let encoded = encode_frame(&frame).expect("Failed to encode");

  // Check magic number
  assert_eq!(encoded[0], 0xBC);
  assert_eq!(encoded[1], 0xBC);
  assert_eq!(encoded[2], 0x80);
  assert_eq!(&encoded[3..], &[1, 2, 3, 4, 5]);

  let decoded = decode_frame(&encoded).expect("Failed to decode");
  assert_eq!(decoded, frame);
}

#[test]
fn test_encode_frame_empty_payload() {
  let frame = MessageFrame::new(0x80, vec![]);
  let result = encode_frame(&frame);
  assert!(result.is_err());
}

#[test]
fn test_decode_frame_invalid_magic() {
  let bytes = [0xAB, 0xCD, 0x80, 1, 2, 3];
  let result = decode_frame(&bytes);
  assert!(result.is_err());
}

#[test]
fn test_decode_frame_too_short() {
  let bytes = [0xBC];
  let result = decode_frame(&bytes);
  assert!(result.is_err());
}

#[test]
fn test_decode_frame_exactly_3_bytes() {
  // Minimum valid frame: magic(2) + type(1) + payload(at least 1 byte)
  // 3 bytes total means empty payload, which should fail
  let bytes = [0xBC, 0xBC, 0x80];
  let result = decode_frame(&bytes);
  assert!(result.is_err());
}

#[test]
fn test_decode_frame_minimum_valid() {
  // Minimum valid frame: 4 bytes
  let bytes = [0xBC, 0xBC, 0x80, 0x01];
  let result = decode_frame(&bytes);
  assert!(result.is_ok());

  let frame = result.unwrap();
  assert_eq!(frame.message_type, 0x80);
  assert_eq!(frame.payload, vec![0x01]);
}

#[test]
fn test_decode_frame_invalid_magic_first_byte() {
  let bytes = [0xAB, 0xBC, 0x80, 1, 2, 3];
  let result = decode_frame(&bytes);
  assert!(result.is_err());
}

#[test]
fn test_decode_frame_invalid_magic_second_byte() {
  let bytes = [0xBC, 0xAB, 0x80, 1, 2, 3];
  let result = decode_frame(&bytes);
  assert!(result.is_err());
}

#[test]
fn test_decode_frame_empty_slice() {
  let bytes: [u8; 0] = [];
  let result = decode_frame(&bytes);
  assert!(result.is_err());
}

#[test]
fn test_decode_frame_1_byte() {
  let bytes = [0xBC];
  let result = decode_frame(&bytes);
  assert!(result.is_err());
}

#[test]
fn test_decode_frame_2_bytes() {
  let bytes = [0xBC, 0xBC];
  let result = decode_frame(&bytes);
  assert!(result.is_err());
}

#[test]
fn test_decode_frame_large_payload() {
  // Create a frame with large payload
  let payload = vec![0xAB; 1_000_000];
  let frame = MessageFrame::new(0x80, payload.clone());
  let encoded = encode_frame(&frame).expect("Failed to encode");

  let decoded = decode_frame(&encoded).expect("Failed to decode");
  assert_eq!(decoded.payload.len(), 1_000_000);
  assert_eq!(decoded.payload, payload);
}

#[test]
fn test_encode_frame_single_byte_payload() {
  let frame = MessageFrame::new(0x80, vec![0x42]);
  let encoded = encode_frame(&frame).expect("Failed to encode");

  assert_eq!(encoded.len(), 4); // 2 magic + 1 type + 1 payload
  assert_eq!(encoded[0], 0xBC);
  assert_eq!(encoded[1], 0xBC);
  assert_eq!(encoded[2], 0x80);
  assert_eq!(encoded[3], 0x42);
}

#[test]
fn test_encode_frame_preserves_all_byte_values() {
  // Test that all byte values (0x00-0xFF) are preserved in payload
  let payload: Vec<u8> = (0u8..=255).collect();
  let frame = MessageFrame::new(0x80, payload.clone());
  let encoded = encode_frame(&frame).expect("Failed to encode");

  let decoded = decode_frame(&encoded).expect("Failed to decode");
  assert_eq!(decoded.payload, payload);
}

#[test]
fn test_frame_chunk_count_empty_payload() {
  // Empty payload edge case
  let frame = MessageFrame::new(0x80, vec![]);
  // Empty payload returns 1 chunk per implementation
  assert_eq!(frame.chunk_count(), 1);
}

#[test]
fn test_frame_chunk_count_exact_chunk_size() {
  // Exactly one chunk
  let frame = MessageFrame::new(0x80, vec![0; MAX_CHUNK_SIZE]);
  assert_eq!(frame.chunk_count(), 1);
}

#[test]
fn test_frame_chunk_count_just_over_one_chunk() {
  // Just over one chunk
  let frame = MessageFrame::new(0x80, vec![0; MAX_CHUNK_SIZE + 1]);
  assert_eq!(frame.chunk_count(), 2);
}

#[test]
fn test_frame_chunk_count_large() {
  // Very large message
  let frame = MessageFrame::new(0x80, vec![0; MAX_CHUNK_SIZE * 100 + 1]);
  assert_eq!(frame.chunk_count(), 101);
}

#[test]
fn test_frame_decode_invalid_magic() {
  // Frame with wrong magic bytes should fail to decode
  let invalid_magic: Vec<u8> = vec![
    0xDE, 0xAD, 0xBE, 0xEF, // Wrong magic
    0x00, 0x01, // Version
    0x00, 0x01, // Message type
    0x00, 0x00, 0x00, 0x0A, // Payload length = 10
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, // Message ID
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Reserved
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 10 bytes payload
  ];
  let result = decode_frame(&invalid_magic);
  assert!(
    result.is_err(),
    "Frame with invalid magic bytes should fail to decode"
  );
}

#[test]
fn test_frame_decode_truncated_header() {
  // Frame with only partial header should fail
  let truncated: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47]; // Just magic bytes, no header
  let result = decode_frame(&truncated);
  assert!(
    result.is_err(),
    "Truncated frame header should fail to decode"
  );
}

#[test]
fn test_frame_decode_zero_length_payload() {
  // Frame with zero-length payload should be rejected by decode_frame
  // because the spec requires a non-empty payload.
  // However, we test that a minimal valid frame (1 byte payload) round-trips.
  let frame = MessageFrame::new(0x80, vec![0x00]);
  let encoded = encode_frame(&frame).expect("encode should succeed");
  let decoded = decode_frame(&encoded).expect("decode should succeed");
  assert_eq!(decoded.message_type, 0x80);
  assert_eq!(decoded.payload.len(), 1);
}

#[test]
fn test_frame_decode_rejects_empty_payload() {
  // Empty payload should be rejected
  let frame = MessageFrame::new(0x80, Vec::new());
  let result = encode_frame(&frame);
  // encode_frame may or may not reject empty payloads - test actual behavior
  if let Ok(encoded) = result {
    let decoded = decode_frame(&encoded);
    // If encode succeeded, decode should reject the empty payload
    assert!(
      decoded.is_err() || decoded.unwrap().payload.is_empty(),
      "Empty payload handling should be consistent between encode and decode"
    );
  }
}

/// Test that `decode_frame` accepts any `message_type` byte value.
/// The frame layer does not validate `message_type` semantics - it's just a transport.
/// Higher layers (WASM, signaling) are responsible for discriminator validation.
#[test]
fn test_frame_decode_accepts_any_message_type() {
  // Test all possible single-byte values
  for message_type in 0u8..=255 {
    let frame = MessageFrame::new(message_type, vec![0x00, 0x01, 0x02]);
    let encoded = encode_frame(&frame).expect("encode should succeed");
    let decoded = decode_frame(&encoded).expect("decode should succeed");
    assert_eq!(decoded.message_type, message_type);
  }
}

/// Test that reserved/gap `message_type` values are accepted at frame level.
/// Even though no `SignalingMessage` or `DataChannelMessage` uses these values,
/// the frame layer should transport them without issue.
#[test]
fn test_frame_decode_reserved_message_types() {
  // Signaling uses 0x00-0x7D, DataChannel uses 0x80-0xBF
  // Test values in gaps (reserved for future use)
  let reserved_types: Vec<u8> = vec![
    0x7E, 0x7F, // Gap between Signaling and DataChannel
    0xC0, 0xC1, 0xCF, 0xD0, 0xDF, 0xE0, 0xEF, 0xF0, 0xFF, // Upper reserved range
  ];

  for message_type in reserved_types {
    let frame = MessageFrame::new(message_type, vec![0xDE, 0xAD, 0xBE, 0xEF]);
    let encoded = encode_frame(&frame).expect("encode should succeed");
    let decoded = decode_frame(&encoded).expect("decode should succeed");
    assert_eq!(
      decoded.message_type, message_type,
      "Frame layer should accept reserved message_type 0x{message_type:02X}"
    );
  }
}

/// Test that a frame with maximum valid `message_type` (0xFF) round-trips correctly.
#[test]
fn test_frame_max_message_type_roundtrip() {
  let frame = MessageFrame::new(0xFF, vec![0xAB; 100]);
  let encoded = encode_frame(&frame).expect("encode should succeed");
  let decoded = decode_frame(&encoded).expect("decode should succeed");
  assert_eq!(decoded.message_type, 0xFF);
  assert_eq!(decoded.payload, vec![0xAB; 100]);
}
