//! Message chunking and size limit tests.

use super::*;

#[test]
fn test_message_frame_chunking_threshold() {
  // Message at exactly the threshold should not need chunking
  let payload_at_threshold = vec![0u8; CHUNKING_THRESHOLD];
  let frame_at_threshold = MessageFrame::new(0x80, payload_at_threshold);
  assert!(!frame_at_threshold.needs_chunking());

  // Message just above the threshold should need chunking
  let payload_above_threshold = vec![0u8; CHUNKING_THRESHOLD + 1];
  let frame_above_threshold = MessageFrame::new(0x80, payload_above_threshold);
  assert!(frame_above_threshold.needs_chunking());
}

#[test]
fn test_max_chunk_size_constant() {
  // Verify MAX_CHUNK_SIZE is 64KB
  assert_eq!(MAX_CHUNK_SIZE, 64 * 1024);
}

#[test]
fn test_chunk_count_calculation() {
  // 64KB message = 1 chunk
  let frame = MessageFrame::new(0x80, vec![0u8; 64 * 1024]);
  assert_eq!(frame.chunk_count(), 1);

  // 64KB + 1 byte = 2 chunks
  let frame = MessageFrame::new(0x80, vec![0u8; 64 * 1024 + 1]);
  assert_eq!(frame.chunk_count(), 2);

  // 128KB = 2 chunks
  let frame = MessageFrame::new(0x80, vec![0u8; 128 * 1024]);
  assert_eq!(frame.chunk_count(), 2);

  // 128KB + 1 byte = 3 chunks
  let frame = MessageFrame::new(0x80, vec![0u8; 128 * 1024 + 1]);
  assert_eq!(frame.chunk_count(), 3);
}

#[test]
fn test_max_reassembly_buffers_limit() {
  assert_eq!(MAX_REASSEMBLY_BUFFERS, 10);
}

#[test]
fn test_reassembly_timeout_seconds() {
  assert_eq!(REASSEMBLY_TIMEOUT_SECS, 30);
}

#[test]
fn test_large_message_chunk_and_reassemble() {
  // Create a 200KB message
  let payload: Vec<u8> = (0u32..200 * 1024).map(|i| (i % 256) as u8).collect();
  let frame = MessageFrame::new(0x80, payload.clone());

  // Chunk it
  let chunks = chunk_message(&frame).expect("Failed to chunk");
  assert!(chunks.len() >= 3); // 200KB should produce at least 3 chunks (64KB + 64KB + 72KB = 4 chunks actually)

  // Verify all chunks have the same message_id
  let message_id = chunks[0].header.message_id;
  for chunk in &chunks {
    assert_eq!(chunk.header.message_id, message_id);
  }

  // Reassemble using ChunkManager
  let mut manager = ChunkManager::new();
  for chunk in &chunks {
    let result = manager
      .process_chunk(chunk, 0x80)
      .expect("Failed to process");
    if let Some((msg_type, data)) = result {
      assert_eq!(msg_type, 0x80);
      let data: Vec<u8> = data;
      assert_eq!(data.len(), payload.len());
      assert_eq!(data, payload);
    }
  }
}

#[test]
fn test_chunk_size_boundary() {
  // Test chunk at exactly MAX_CHUNK_SIZE
  let _payload = vec![0xABu8; MAX_CHUNK_SIZE];
  let frame = MessageFrame::new(0x80, vec![0; MAX_CHUNK_SIZE + 1]);

  let chunks = chunk_message(&frame).expect("Failed to chunk");
  assert_eq!(chunks.len(), 2);

  // First chunk should be exactly MAX_CHUNK_SIZE
  assert_eq!(chunks[0].data.len(), MAX_CHUNK_SIZE);

  // Second chunk should be 1 byte
  assert_eq!(chunks[1].data.len(), 1);
}
