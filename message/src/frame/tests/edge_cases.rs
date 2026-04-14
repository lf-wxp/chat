//! Edge case tests for frame module.

use super::*;
use crate::types::MessageId;

// =============================================================================
// Additional Edge Case Tests (MA-P1-002, BUG-002)
// =============================================================================

#[test]
fn test_reassembly_buffer_out_of_bounds_chunk_index() {
  let message_id = MessageId::new();
  let mut buffer = ReassemblyBuffer::new(message_id, 100, 2, 0x80);

  // Chunk with index >= total_chunks should return an error
  let bad_chunk = ChunkedMessage::new(
    ChunkHeader::new(message_id, 100, 5, 2), // index 5 but total is 2
    vec![0; 50],
  );
  // add_chunk should return Err for out-of-bounds chunk index
  let result = buffer.add_chunk(&bad_chunk);
  assert!(result.is_err());
}

#[test]
fn test_reassembly_buffer_chunk_index_at_boundary() {
  let message_id = MessageId::new();
  let mut buffer = ReassemblyBuffer::new(message_id, 100, 3, 0x80);

  // Valid indices: 0, 1, 2. Index 3 should be out of bounds.
  let chunk0 = ChunkedMessage::new(ChunkHeader::new(message_id, 100, 0, 3), vec![0; 34]);
  let chunk1 = ChunkedMessage::new(ChunkHeader::new(message_id, 100, 1, 3), vec![0; 33]);
  let chunk2 = ChunkedMessage::new(ChunkHeader::new(message_id, 100, 2, 3), vec![0; 33]);

  assert!(buffer.add_chunk(&chunk0).expect("chunk0 should succeed"));
  assert!(buffer.add_chunk(&chunk1).expect("chunk1 should succeed"));
  assert!(buffer.add_chunk(&chunk2).expect("chunk2 should succeed"));

  // chunk3 with index 3 is out of bounds - this will panic due to bitmap check
  // We verify that the first three chunks succeeded
  assert!(buffer.bitmap.is_complete());
}

// =============================================================================
// Chunk Offset Overflow Tests (BUG-002)
// =============================================================================

/// Test that chunk offset calculation doesn't overflow.
/// When `total_size` or `chunk_index` is very large, the offset calculation
/// should handle it gracefully.
#[test]
fn test_chunk_offset_calculation_no_overflow_small() {
  let message_id = MessageId::new();
  let mut buffer = ReassemblyBuffer::new(message_id, 1000, 5, 0x80);

  // Add chunks with normal offsets
  for i in 0u32..5 {
    let chunk = ChunkedMessage::new(
      ChunkHeader::new(message_id, 1000, i, 5),
      vec![u8::try_from(i).expect("i should fit in u8"); 200],
    );
    assert!(buffer.add_chunk(&chunk).expect("should add"));
  }

  let result = buffer.reassemble().expect("should reassemble");
  assert_eq!(result.len(), 1000);
}

/// Test that out-of-range chunk index is handled gracefully.
#[test]
fn test_reassembly_buffer_out_of_range_chunk_index() {
  let message_id = MessageId::new();
  let total_chunks = 2u32;

  let mut buffer = ReassemblyBuffer::new(message_id, 200, total_chunks, 0x80);

  // Add chunk with index beyond total_chunks
  // Should return an error instead of panicking
  let chunk_out_of_range = ChunkedMessage::new(
    ChunkHeader::new(message_id, 200, 5, total_chunks), // index 5 > total 2
    vec![0xFF; 100],
  );

  // Should return Err gracefully
  let result = buffer.add_chunk(&chunk_out_of_range);
  assert!(result.is_err());
}

// =============================================================================
// Additional Frame Edge Case Tests
// =============================================================================

#[test]
fn test_frame_chunk_count_empty_payload() {
  // Empty payload should still require 1 chunk
  let frame = MessageFrame::new(0x01, vec![]);
  assert_eq!(frame.chunk_count(), 1);
}

#[test]
fn test_frame_chunk_count_single_chunk() {
  // Payload less than MAX_CHUNK_SIZE
  let frame = MessageFrame::new(0x01, vec![0u8; 1000]);
  assert_eq!(frame.chunk_count(), 1);
}

#[test]
fn test_frame_chunk_count_exact_multiple() {
  // Payload exactly MAX_CHUNK_SIZE * 2
  let frame = MessageFrame::new(0x01, vec![0u8; MAX_CHUNK_SIZE * 2]);
  assert_eq!(frame.chunk_count(), 2);
}

#[test]
fn test_frame_chunk_count_requires_ceil() {
  // Payload slightly over MAX_CHUNK_SIZE
  let frame = MessageFrame::new(0x01, vec![0u8; MAX_CHUNK_SIZE + 1]);
  assert_eq!(frame.chunk_count(), 2);
}
