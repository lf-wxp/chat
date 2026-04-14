//! `ChunkHeader`, `ChunkBitmap`, and `ChunkedMessage` tests.

use super::*;
use crate::types::MessageId;

// =============================================================================
// Chunk Header Tests
// =============================================================================

#[test]
fn test_chunk_header() {
  let header = ChunkHeader::new(MessageId::nil(), 1_000_000, 0, 10);
  assert_eq!(header.total_size, 1_000_000);
  assert_eq!(header.chunk_index, 0);
  assert_eq!(header.total_chunks, 10);
  assert!(!header.is_last_chunk());

  let last_header = ChunkHeader::new(MessageId::nil(), 1_000_000, 9, 10);
  assert!(last_header.is_last_chunk());
}

#[test]
fn test_chunk_header_expected_size() {
  // First chunk of 150KB (3 chunks total)
  let header = ChunkHeader::new(MessageId::nil(), 150 * 1024, 0, 3);
  assert_eq!(header.expected_chunk_size(), MAX_CHUNK_SIZE);

  // Last chunk of 150KB (3 chunks: 64KB + 64KB + 22KB)
  let last_header = ChunkHeader::new(MessageId::nil(), 150 * 1024, 2, 3);
  assert_eq!(last_header.expected_chunk_size(), 22 * 1024);
}

#[test]
fn test_chunk_header_is_last_chunk_single_chunk() {
  let header = ChunkHeader::new(MessageId::nil(), 100, 0, 1);
  assert!(header.is_last_chunk());
}

#[test]
fn test_chunk_header_expected_chunk_size_single_chunk() {
  // Single chunk message
  let header = ChunkHeader::new(MessageId::nil(), 50000, 0, 1);
  assert_eq!(header.expected_chunk_size(), 50000);
}

#[test]
fn test_chunk_header_expected_chunk_size_exact_multiple() {
  // 2 chunks of exactly 64KB each = 128KB total
  let header_first = ChunkHeader::new(
    MessageId::nil(),
    u32::try_from(MAX_CHUNK_SIZE).unwrap() * 2,
    0,
    2,
  );
  assert_eq!(header_first.expected_chunk_size(), MAX_CHUNK_SIZE);

  let header_last = ChunkHeader::new(
    MessageId::nil(),
    u32::try_from(MAX_CHUNK_SIZE).unwrap() * 2,
    1,
    2,
  );
  assert_eq!(header_last.expected_chunk_size(), MAX_CHUNK_SIZE);
}

#[test]
fn test_chunk_header_total_size_zero() {
  let header = ChunkHeader::new(MessageId::nil(), 0, 0, 1);
  assert_eq!(header.total_size, 0);
  assert_eq!(header.expected_chunk_size(), 0);
}

#[test]
fn test_chunk_header_expected_size_first_chunk() {
  // First chunk should always be MAX_CHUNK_SIZE
  let header = ChunkHeader::new(
    MessageId::new(),
    u32::try_from(MAX_CHUNK_SIZE).unwrap() * 3 + 1000,
    0,
    4,
  );
  assert_eq!(header.expected_chunk_size(), MAX_CHUNK_SIZE);
}

#[test]
fn test_chunk_header_expected_size_middle_chunk() {
  // Middle chunk should always be MAX_CHUNK_SIZE
  let header = ChunkHeader::new(
    MessageId::new(),
    u32::try_from(MAX_CHUNK_SIZE).unwrap() * 3 + 1000,
    1,
    4,
  );
  assert_eq!(header.expected_chunk_size(), MAX_CHUNK_SIZE);
}

#[test]
fn test_chunk_header_expected_size_last_chunk() {
  // Last chunk may be smaller
  let header = ChunkHeader::new(
    MessageId::new(),
    u32::try_from(MAX_CHUNK_SIZE).unwrap() * 3 + 1000,
    3,
    4,
  );
  assert_eq!(header.expected_chunk_size(), 1000);
}

#[test]
fn test_chunk_header_max_values() {
  let message_id = MessageId::new();
  // Use large but valid values
  let header = ChunkHeader::new(message_id, u32::MAX, u32::MAX - 1, u32::MAX);

  // The header should be created without panic
  assert_eq!(header.total_size, u32::MAX);
  assert_eq!(header.chunk_index, u32::MAX - 1);
  assert_eq!(header.total_chunks, u32::MAX);
}

// =============================================================================
// Chunk Bitmap Tests
// =============================================================================

#[test]
fn test_chunk_bitmap_basic() {
  let mut bitmap = ChunkBitmap::new(10);

  assert_eq!(bitmap.received_count(), 0);
  assert!(!bitmap.is_complete());

  bitmap.set_received(0).unwrap();
  assert!(bitmap.is_received(0).unwrap());
  assert_eq!(bitmap.received_count(), 1);

  bitmap.set_received(5).unwrap();
  assert!(bitmap.is_received(5).unwrap());
  assert_eq!(bitmap.received_count(), 2);

  // Setting again should not increase count (but also not error)
  bitmap.set_received(5).unwrap();
  assert_eq!(bitmap.received_count(), 2);
}

#[test]
fn test_chunk_bitmap_completion() {
  let mut bitmap = ChunkBitmap::new(3);
  bitmap.set_received(0).unwrap();
  bitmap.set_received(1).unwrap();
  bitmap.set_received(2).unwrap();

  assert!(bitmap.is_complete());
  assert_eq!(bitmap.completion_percentage(), 100);
}

#[test]
fn test_chunk_bitmap_missing() {
  let mut bitmap = ChunkBitmap::new(5);
  bitmap.set_received(0).unwrap();
  bitmap.set_received(2).unwrap();
  bitmap.set_received(4).unwrap();

  let missing = bitmap.missing_chunks();
  assert_eq!(missing, vec![1, 3]);
}

#[test]
fn test_chunk_bitmap_large() {
  let mut bitmap = ChunkBitmap::new(100);

  // Set some chunks across u64 boundaries
  bitmap.set_received(0).unwrap();
  bitmap.set_received(63).unwrap();
  bitmap.set_received(64).unwrap();
  bitmap.set_received(99).unwrap();

  assert!(bitmap.is_received(0).unwrap());
  assert!(bitmap.is_received(63).unwrap());
  assert!(bitmap.is_received(64).unwrap());
  assert!(bitmap.is_received(99).unwrap());
  assert!(!bitmap.is_received(50).unwrap());
  assert_eq!(bitmap.received_count(), 4);
}

#[test]
fn test_chunk_bitmap_zero_chunks() {
  let bitmap = ChunkBitmap::new(0);
  assert_eq!(bitmap.received_count(), 0);
  // With 0 chunks, it should be considered complete
  assert!(bitmap.is_complete());
  assert_eq!(bitmap.completion_percentage(), 100);
  assert!(bitmap.missing_chunks().is_empty());
}

#[test]
fn test_chunk_bitmap_single_chunk() {
  let mut bitmap = ChunkBitmap::new(1);
  assert!(!bitmap.is_complete());

  bitmap.set_received(0).unwrap();
  assert!(bitmap.is_received(0).unwrap());
  assert!(bitmap.is_complete());
  assert_eq!(bitmap.completion_percentage(), 100);
}

#[test]
fn test_chunk_bitmap_64_chunks_boundary() {
  // Test at the u64 boundary (64 bits per u64)
  let mut bitmap = ChunkBitmap::new(64);

  // Set chunks across the first u64 boundary
  bitmap.set_received(0).unwrap();
  bitmap.set_received(63).unwrap();

  assert!(bitmap.is_received(0).unwrap());
  assert!(bitmap.is_received(63).unwrap());
  assert!(!bitmap.is_received(31).unwrap());
  assert_eq!(bitmap.received_count(), 2);

  // Complete the first 64
  for i in 0..64 {
    bitmap.set_received(i).unwrap();
  }
  assert!(bitmap.is_complete());
}

#[test]
fn test_chunk_bitmap_65_chunks_crosses_boundary() {
  // Test crossing u64 boundary (65 chunks requires 2 u64s)
  let mut bitmap = ChunkBitmap::new(65);

  // Set chunk at index 64 (second u64, first bit)
  bitmap.set_received(64).unwrap();
  assert!(bitmap.is_received(64).unwrap());
  assert!(!bitmap.is_received(0).unwrap());
  assert_eq!(bitmap.received_count(), 1);
}

#[test]
fn test_chunk_bitmap_completion_percentage_partial() {
  let mut bitmap = ChunkBitmap::new(10);

  assert_eq!(bitmap.completion_percentage(), 0);

  bitmap.set_received(0).unwrap();
  assert_eq!(bitmap.completion_percentage(), 10);

  bitmap.set_received(1).unwrap();
  assert_eq!(bitmap.completion_percentage(), 20);

  // Set 5 chunks total
  bitmap.set_received(2).unwrap();
  bitmap.set_received(3).unwrap();
  bitmap.set_received(4).unwrap();
  assert_eq!(bitmap.completion_percentage(), 50);
}

#[test]
fn test_chunk_bitmap_large_number_of_chunks() {
  // Test with many chunks
  let mut bitmap = ChunkBitmap::new(200);

  for i in 0..200 {
    bitmap.set_received(i).unwrap();
  }

  assert!(bitmap.is_complete());
  assert_eq!(bitmap.received_count(), 200);
  assert!(bitmap.missing_chunks().is_empty());
}

#[test]
fn test_chunk_bitmap_set_received_out_of_bounds() {
  let mut bitmap = ChunkBitmap::new(5);
  let result = bitmap.set_received(5); // Index 5 is out of bounds for 5 chunks (0-4)
  assert!(result.is_err());
}

#[test]
fn test_chunk_bitmap_is_received_out_of_bounds() {
  let bitmap = ChunkBitmap::new(5);
  let result = bitmap.is_received(10); // Way out of bounds
  assert!(result.is_err());
}

#[test]
fn test_chunk_bitmap_all_missing_when_none_received() {
  let bitmap = ChunkBitmap::new(5);
  let missing = bitmap.missing_chunks();
  assert_eq!(missing, vec![0, 1, 2, 3, 4]);
}

#[test]
fn test_chunk_bitmap_completion_percentage_zero_chunks() {
  // Edge case: bitmap with 0 chunks
  let bitmap = ChunkBitmap::new(0);
  assert_eq!(bitmap.completion_percentage(), 100);
}

#[test]
fn test_chunk_bitmap_missing_chunks_all_missing() {
  let bitmap = ChunkBitmap::new(5);
  let missing = bitmap.missing_chunks();
  assert_eq!(missing, vec![0, 1, 2, 3, 4]);
}

#[test]
fn test_chunk_bitmap_missing_chunks_none_missing() {
  let mut bitmap = ChunkBitmap::new(3);
  bitmap.set_received(0).unwrap();
  bitmap.set_received(1).unwrap();
  bitmap.set_received(2).unwrap();
  let missing = bitmap.missing_chunks();
  assert!(missing.is_empty());
}

#[test]
fn test_chunk_bitmap_missing_chunks_partial() {
  let mut bitmap = ChunkBitmap::new(5);
  bitmap.set_received(1).unwrap();
  bitmap.set_received(3).unwrap();
  let missing = bitmap.missing_chunks();
  assert_eq!(missing, vec![0, 2, 4]);
}

// =============================================================================
// Chunked Message Tests
// =============================================================================

#[test]
fn test_chunked_message_encode_decode() {
  let chunk = ChunkedMessage::new(
    ChunkHeader::new(MessageId::new(), 1000, 0, 2),
    vec![1, 2, 3, 4, 5],
  );

  let encoded = chunk.encode().expect("Failed to encode");
  let decoded = ChunkedMessage::decode(&encoded).expect("Failed to decode");

  assert_eq!(decoded, chunk);
}

#[test]
fn test_create_chunk() {
  let message_id = MessageId::new();
  let chunk = create_chunk(message_id, 0x80, &[1, 2, 3], 0, 10, 1000);

  assert_eq!(chunk.header.message_id, message_id);
  assert_eq!(chunk.header.total_size, 1000);
  assert_eq!(chunk.header.chunk_index, 0);
  assert_eq!(chunk.header.total_chunks, 10);
  assert_eq!(chunk.data, vec![1, 2, 3]);
}

#[test]
fn test_chunk_message() {
  let frame = MessageFrame::new(0x80, vec![0xAB; 150 * 1024]);
  let chunks = chunk_message(&frame).expect("Failed to chunk");

  assert_eq!(chunks.len(), 3);
  assert_eq!(chunks[0].header.total_chunks, 3);
  assert_eq!(chunks[0].header.chunk_index, 0);
  assert_eq!(chunks[1].header.chunk_index, 1);
  assert_eq!(chunks[2].header.chunk_index, 2);

  // All chunks should have the same message_id
  let message_id = chunks[0].header.message_id;
  for chunk in &chunks {
    assert_eq!(chunk.header.message_id, message_id);
  }
}

#[test]
fn test_chunk_message_too_small() {
  let frame = MessageFrame::new(0x80, vec![0; 1024]);
  let result = chunk_message(&frame);
  assert!(result.is_err());
}

#[test]
fn test_chunked_message_round_trip_with_max_data() {
  let header = ChunkHeader::new(
    MessageId::new(),
    u32::try_from(MAX_CHUNK_SIZE).unwrap() * 2,
    0,
    2,
  );
  let chunk = ChunkedMessage::new(header, vec![0xFF; MAX_CHUNK_SIZE]);

  let encoded = chunk.encode().expect("Failed to encode");
  let decoded = ChunkedMessage::decode(&encoded).expect("Failed to decode");

  assert_eq!(decoded, chunk);
}

#[test]
fn test_chunked_message_round_trip_preserves_all_fields() {
  let message_id = MessageId::new();
  let header = ChunkHeader::new(message_id, 12345, 7, 15);
  let data = vec![0xDE, 0xAD, 0xBE, 0xEF];
  let chunk = ChunkedMessage::new(header, data);

  let encoded = chunk.encode().expect("Failed to encode");
  let decoded = ChunkedMessage::decode(&encoded).expect("Failed to decode");

  assert_eq!(decoded.header.message_id, message_id);
  assert_eq!(decoded.header.total_size, 12345);
  assert_eq!(decoded.header.chunk_index, 7);
  assert_eq!(decoded.header.total_chunks, 15);
  assert_eq!(decoded.data, vec![0xDE, 0xAD, 0xBE, 0xEF]);
}

#[test]
fn test_chunk_message_exactly_at_threshold() {
  // Message exactly at chunking threshold should NOT need chunking
  let frame = MessageFrame::new(0x80, vec![0; CHUNKING_THRESHOLD]);
  assert!(!frame.needs_chunking());

  let result = chunk_message(&frame);
  assert!(result.is_err());
}

#[test]
fn test_chunk_message_one_byte_over_threshold() {
  // Message one byte over threshold should need chunking
  let frame = MessageFrame::new(0x80, vec![0; CHUNKING_THRESHOLD + 1]);
  assert!(frame.needs_chunking());

  let chunks = chunk_message(&frame).expect("Failed to chunk");
  assert_eq!(chunks.len(), 2);
}

#[test]
fn test_chunk_message_exact_multiple_of_chunk_size() {
  // Exactly 3 chunks of 64KB
  let frame = MessageFrame::new(0x80, vec![0xAB; MAX_CHUNK_SIZE * 3]);
  let chunks = chunk_message(&frame).expect("Failed to chunk");

  assert_eq!(chunks.len(), 3);
  for chunk in &chunks {
    assert_eq!(chunk.data.len(), MAX_CHUNK_SIZE);
  }
}

#[test]
fn test_chunk_message_consistency() {
  let payload: Vec<u8> = (0..=255u8).cycle().take(100_000).collect();
  let frame = MessageFrame::new(0x80, payload.clone());
  let chunks = chunk_message(&frame).expect("Failed to chunk");

  // Verify all chunks have same message_id and consistent metadata
  let message_id = chunks[0].header.message_id;
  let total_size = chunks[0].header.total_size;
  let total_chunks = chunks[0].header.total_chunks;

  for (i, chunk) in chunks.iter().enumerate() {
    assert_eq!(chunk.header.message_id, message_id);
    assert_eq!(chunk.header.total_size, total_size);
    assert_eq!(chunk.header.total_chunks, total_chunks);
    assert_eq!(chunk.header.chunk_index, u32::try_from(i).unwrap());
  }
}

#[test]
fn test_chunked_message_edge_cases() {
  let message_id = MessageId::new();

  // Minimum valid chunk
  let min_chunk = ChunkedMessage::new(ChunkHeader::new(message_id, 1, 0, 1), vec![0]);
  assert_eq!(min_chunk.data.len(), 1);

  // Chunk with mismatched data size (implementation should handle)
  let mismatched = ChunkedMessage::new(ChunkHeader::new(message_id, 100, 0, 2), vec![0; 50]);
  // Just verify no panic - implementation may or may not validate
  assert_eq!(mismatched.data.len(), 50);
}

// Import CHUNKING_THRESHOLD for use in tests
use crate::frame::CHUNKING_THRESHOLD;
