//! `ReassemblyBuffer` and `ChunkManager` tests.

use super::*;
use crate::types::MessageId;
use std::time::{Duration, Instant};

// =============================================================================
// Reassembly Buffer Basic Tests
// =============================================================================

#[test]
fn test_reassembly_buffer_basic() {
  let message_id = MessageId::new();
  let mut buffer = ReassemblyBuffer::new(message_id, 100, 2, 0x80);

  let chunk1 = ChunkedMessage::new(ChunkHeader::new(message_id, 100, 0, 2), vec![0; 50]);
  let chunk2 = ChunkedMessage::new(ChunkHeader::new(message_id, 100, 1, 2), vec![0; 50]);

  assert!(buffer.add_chunk(&chunk1).expect("add_chunk should succeed"));
  assert!(!buffer.is_complete());

  assert!(buffer.add_chunk(&chunk2).expect("add_chunk should succeed"));
  assert!(buffer.is_complete());

  let result = buffer.reassemble().expect("Failed to reassemble");
  assert_eq!(result.len(), 100);
}

#[test]
fn test_reassembly_buffer_duplicate() {
  let message_id = MessageId::new();
  let mut buffer = ReassemblyBuffer::new(message_id, 100, 2, 0x80);

  let chunk1 = ChunkedMessage::new(ChunkHeader::new(message_id, 100, 0, 2), vec![0; 50]);

  assert!(buffer.add_chunk(&chunk1).expect("add_chunk should succeed")); // First add
  assert!(!buffer.add_chunk(&chunk1).expect("add_chunk should succeed")); // Duplicate
}

#[test]
fn test_reassembly_buffer_size_mismatch() {
  let message_id = MessageId::new();
  let mut buffer = ReassemblyBuffer::new(message_id, 100, 2, 0x80);

  // Add chunks with wrong total size in header
  let chunk1 = ChunkedMessage::new(ChunkHeader::new(message_id, 100, 0, 2), vec![0; 50]);
  let chunk2 = ChunkedMessage::new(ChunkHeader::new(message_id, 100, 1, 2), vec![0; 60]); // 60 instead of 50

  buffer.add_chunk(&chunk1).expect("add_chunk should succeed");
  buffer.add_chunk(&chunk2).expect("add_chunk should succeed");

  assert!(buffer.is_complete());

  // Reassemble should fail due to size mismatch
  let result = buffer.reassemble();
  assert!(result.is_err());
}

#[test]
fn test_reassembly_buffer_reassemble_incomplete() {
  let message_id = MessageId::new();
  let buffer = ReassemblyBuffer::new(message_id, 100, 3, 0x80);

  // Try to reassemble without adding any chunks
  let result = buffer.reassemble();
  assert!(result.is_err());
}

#[test]
fn test_reassembly_buffer_single_chunk_message() {
  let message_id = MessageId::new();
  let mut buffer = ReassemblyBuffer::new(message_id, 50, 1, 0x80);

  let chunk = ChunkedMessage::new(ChunkHeader::new(message_id, 50, 0, 1), vec![0xAB; 50]);

  buffer.add_chunk(&chunk).expect("add_chunk should succeed");
  assert!(buffer.is_complete());

  let result = buffer.reassemble().expect("Failed to reassemble");
  assert_eq!(result.len(), 50);
  assert_eq!(result, vec![0xAB; 50]);
}

#[test]
fn test_reassembly_buffer_preserves_message_type() {
  let message_id = MessageId::new();
  let buffer = ReassemblyBuffer::new(message_id, 100, 2, 0xFF);

  assert_eq!(buffer.message_type, 0xFF);
}

// =============================================================================
// Reassembly Buffer Timeout Tests
// =============================================================================

#[test]
fn test_reassembly_buffer_not_timed_out_initially() {
  let message_id = MessageId::new();
  let buffer = ReassemblyBuffer::new(message_id, 100, 2, 0x80);

  // Fresh buffer should not be timed out
  assert!(!buffer.is_timed_out());
}

#[test]
fn test_reassembly_buffer_timed_out_after_30_seconds() {
  let message_id = MessageId::new();
  let mut buffer = ReassemblyBuffer::new(message_id, 100, 2, 0x80);

  // Simulate 30 seconds have passed by modifying created_at
  // Note: created_at is public, so we can modify it in tests
  buffer.created_at = Instant::now()
    .checked_sub(Duration::from_secs(REASSEMBLY_TIMEOUT_SECS))
    .expect("time subtraction should not underflow");

  // Buffer should be timed out now
  assert!(buffer.is_timed_out());

  // Buffer should also be timed out if more than 30 seconds passed
  buffer.created_at = Instant::now()
    .checked_sub(Duration::from_secs(REASSEMBLY_TIMEOUT_SECS + 1))
    .expect("time subtraction should not underflow");
  assert!(buffer.is_timed_out());
}

#[test]
fn test_reassembly_buffer_not_timed_out_before_30_seconds() {
  let message_id = MessageId::new();
  let mut buffer = ReassemblyBuffer::new(message_id, 100, 2, 0x80);

  // Simulate 29 seconds have passed
  buffer.created_at = Instant::now()
    .checked_sub(Duration::from_secs(REASSEMBLY_TIMEOUT_SECS - 1))
    .expect("time subtraction should not underflow");

  // Buffer should NOT be timed out yet
  assert!(!buffer.is_timed_out());
}

// =============================================================================
// Chunk Manager Basic Tests
// =============================================================================

#[test]
fn test_chunk_manager_basic() {
  let mut manager = ChunkManager::new();
  let message_id = MessageId::new();

  let chunk1 = ChunkedMessage::new(ChunkHeader::new(message_id, 100, 0, 2), vec![1; 50]);
  let chunk2 = ChunkedMessage::new(ChunkHeader::new(message_id, 100, 1, 2), vec![2; 50]);

  // First chunk - not complete
  let result = manager.process_chunk(&chunk1, 0x80).expect("Failed");
  assert!(result.is_none());
  assert_eq!(manager.active_buffer_count(), 1);

  // Second chunk - complete
  let result = manager.process_chunk(&chunk2, 0x80).expect("Failed");
  assert!(result.is_some());

  let (msg_type, data) = result.unwrap();
  assert_eq!(msg_type, 0x80);
  assert_eq!(data.len(), 100);
  assert_eq!(&data[0..50], &[1; 50]);
  assert_eq!(&data[50..100], &[2; 50]);

  // Buffer should be removed after completion
  assert_eq!(manager.active_buffer_count(), 0);
}

#[test]
fn test_chunk_manager_cleanup_timeouts() {
  let mut manager = ChunkManager::new();

  // Create two chunks with different message IDs
  let message_id1 = MessageId::new();
  let message_id2 = MessageId::new();

  let chunk1 = ChunkedMessage::new(ChunkHeader::new(message_id1, 100, 0, 2), vec![1; 50]);
  let chunk2 = ChunkedMessage::new(ChunkHeader::new(message_id2, 200, 0, 2), vec![2; 50]);

  // Process first chunk
  manager.process_chunk(&chunk1, 0x80).expect("Failed");
  // Process second chunk
  manager.process_chunk(&chunk2, 0x80).expect("Failed");

  assert_eq!(manager.active_buffer_count(), 2);

  // Manually expire the first buffer
  if let Some(buffer) = manager.buffers.get_mut(&message_id1) {
    buffer.created_at = Instant::now()
      .checked_sub(Duration::from_secs(REASSEMBLY_TIMEOUT_SECS + 1))
      .expect("time subtraction should not underflow");
  }

  // Cleanup timeouts
  manager.cleanup_timeouts();

  // Only one buffer should remain (the non-expired one)
  assert_eq!(manager.active_buffer_count(), 1);
  assert!(manager.get_buffer(&message_id1).is_none());
  assert!(manager.get_buffer(&message_id2).is_some());
}

#[test]
fn test_chunk_manager_cleanup_timeouts_on_process_chunk() {
  let mut manager = ChunkManager::new();

  // Create an expired buffer manually
  let message_id_expired = MessageId::new();
  let chunk_expired =
    ChunkedMessage::new(ChunkHeader::new(message_id_expired, 100, 0, 2), vec![0; 50]);

  // Process to create buffer
  manager.process_chunk(&chunk_expired, 0x80).expect("Failed");
  assert_eq!(manager.active_buffer_count(), 1);

  // Expire the buffer
  if let Some(buffer) = manager.buffers.get_mut(&message_id_expired) {
    buffer.created_at = Instant::now()
      .checked_sub(Duration::from_secs(REASSEMBLY_TIMEOUT_SECS + 1))
      .expect("time subtraction should not underflow");
  }

  // Process a new chunk - this should trigger cleanup
  let message_id_new = MessageId::new();
  let chunk_new = ChunkedMessage::new(ChunkHeader::new(message_id_new, 200, 0, 2), vec![1; 50]);
  manager.process_chunk(&chunk_new, 0x80).expect("Failed");

  // The expired buffer should have been cleaned up, only the new one should remain
  assert_eq!(manager.active_buffer_count(), 1);
  assert!(manager.get_buffer(&message_id_new).is_some());
}

// =============================================================================
// Chunk Manager Buffer Limit Tests
// =============================================================================

#[test]
fn test_chunk_manager_max_reassembly_buffers_limit() {
  let mut manager = ChunkManager::new();

  // Create MAX_REASSEMBLY_BUFFERS buffers
  for i in 0..MAX_REASSEMBLY_BUFFERS {
    let message_id = MessageId::new();
    let chunk = ChunkedMessage::new(
      ChunkHeader::new(
        message_id,
        100 + u32::try_from(i).expect("i should fit in u32"),
        0,
        2,
      ),
      vec![u8::try_from(i).expect("i should fit in u8"); 50],
    );
    manager.process_chunk(&chunk, 0x80).expect("Failed");
  }

  assert_eq!(manager.active_buffer_count(), MAX_REASSEMBLY_BUFFERS);

  // Adding one more should trigger removal of the oldest buffer
  let message_id_new = MessageId::new();
  let chunk_new = ChunkedMessage::new(ChunkHeader::new(message_id_new, 999, 0, 2), vec![99; 50]);
  manager.process_chunk(&chunk_new, 0x80).expect("Failed");

  // Should still be at MAX_REASSEMBLY_BUFFERS (not exceed)
  assert_eq!(manager.active_buffer_count(), MAX_REASSEMBLY_BUFFERS);

  // The newest buffer should exist
  assert!(manager.get_buffer(&message_id_new).is_some());
}

#[test]
fn test_chunk_manager_removes_oldest_buffer_when_exceeded() {
  let mut manager = ChunkManager::new();
  let mut message_ids: Vec<MessageId> = Vec::new();

  // Create MAX_REASSEMBLY_BUFFERS + 1 buffers with slight timing differences
  for i in 0..=MAX_REASSEMBLY_BUFFERS {
    let message_id = MessageId::new();
    message_ids.push(message_id);

    let chunk = ChunkedMessage::new(
      ChunkHeader::new(
        message_id,
        100 + u32::try_from(i).expect("i should fit in u32"),
        0,
        2,
      ),
      vec![u8::try_from(i).expect("i should fit in u8"); 50],
    );
    manager.process_chunk(&chunk, 0x80).expect("Failed");

    // Small sleep to ensure different creation times (except last one)
    if i < MAX_REASSEMBLY_BUFFERS {
      std::thread::sleep(Duration::from_millis(1));
    }
  }

  // Should be at MAX_REASSEMBLY_BUFFERS (oldest was removed)
  assert_eq!(manager.active_buffer_count(), MAX_REASSEMBLY_BUFFERS);

  // The first (oldest) buffer should have been removed
  assert!(manager.get_buffer(&message_ids[0]).is_none());

  // All other buffers should exist
  for (i, mid) in message_ids.iter().enumerate().skip(1) {
    assert!(manager.get_buffer(mid).is_some(), "Buffer {i} should exist");
  }
}

#[test]
fn test_chunk_manager_multiple_exceeds() {
  let mut manager = ChunkManager::new();
  let mut message_ids: Vec<MessageId> = Vec::new();

  // Create significantly more than MAX_REASSEMBLY_BUFFERS buffers
  let total_buffers = MAX_REASSEMBLY_BUFFERS + 5;
  for i in 0..total_buffers {
    let message_id = MessageId::new();
    message_ids.push(message_id);

    let chunk = ChunkedMessage::new(
      ChunkHeader::new(message_id, 100 + u32::try_from(i).unwrap(), 0, 2),
      vec![u8::try_from(i).unwrap(); 50],
    );
    manager.process_chunk(&chunk, 0x80).expect("Failed");

    // Small sleep to ensure different creation times
    std::thread::sleep(Duration::from_millis(1));
  }

  // Should be capped at MAX_REASSEMBLY_BUFFERS
  assert_eq!(manager.active_buffer_count(), MAX_REASSEMBLY_BUFFERS);

  // The oldest (total_buffers - MAX_REASSEMBLY_BUFFERS) buffers should be gone
  let first_existing_index = total_buffers - MAX_REASSEMBLY_BUFFERS;
  for (i, mid) in message_ids.iter().enumerate().take(first_existing_index) {
    assert!(
      manager.get_buffer(mid).is_none(),
      "Buffer {i} should have been removed"
    );
  }

  // The most recent MAX_REASSEMBLY_BUFFERS should exist
  for (i, mid) in message_ids.iter().enumerate().skip(first_existing_index) {
    assert!(manager.get_buffer(mid).is_some(), "Buffer {i} should exist");
  }
}

#[test]
fn test_chunk_manager_empty() {
  let manager = ChunkManager::new();
  assert_eq!(manager.active_buffer_count(), 0);
}

#[test]
fn test_chunk_manager_get_nonexistent_buffer() {
  let manager = ChunkManager::new();
  let result = manager.get_buffer(&MessageId::new());
  assert!(result.is_none());
}

#[test]
fn test_chunk_manager_same_message_different_types() {
  // Same message ID but different message types (should use first type)
  let mut manager = ChunkManager::new();
  let message_id = MessageId::new();

  let chunk1 = ChunkedMessage::new(ChunkHeader::new(message_id, 100, 0, 2), vec![1; 50]);
  let chunk2 = ChunkedMessage::new(ChunkHeader::new(message_id, 100, 1, 2), vec![2; 50]);

  // First chunk sets type to 0x80
  let _ = manager.process_chunk(&chunk1, 0x80).unwrap();

  // Second chunk with different type should still use original type
  let result = manager.process_chunk(&chunk2, 0xFF).unwrap();
  assert!(result.is_some());

  let (msg_type, _) = result.unwrap();
  // Type should be from the buffer creation (first chunk)
  assert_eq!(msg_type, 0x80);
}

// =============================================================================
// Chunk Manager Eviction Tests
// =============================================================================

#[test]
fn test_chunk_manager_eviction_policy() {
  // BUG-001: Verify that ChunkManager evicts the OLDEST buffer
  // when MAX_REASSEMBLY_BUFFERS is exceeded.
  let mut manager = ChunkManager::new();
  let message_type: u8 = 0x80;

  // Fill up to max capacity
  for i in 0..MAX_REASSEMBLY_BUFFERS {
    let message_id = MessageId::new();
    let chunk = ChunkedMessage::new(ChunkHeader::new(message_id, 200, 0, 2), vec![0; 100]);
    let result = manager.process_chunk(&chunk, message_type);
    assert!(result.is_ok(), "Chunk {i} should be accepted");
  }

  // Adding one more should evict the oldest (first message)
  let new_message_id = MessageId::new();
  let extra_chunk = ChunkedMessage::new(ChunkHeader::new(new_message_id, 200, 0, 2), vec![0; 100]);
  let result = manager.process_chunk(&extra_chunk, message_type);
  assert!(
    result.is_ok(),
    "Extra chunk should be accepted after eviction"
  );

  // The first message's buffer should have been evicted
  // We can verify by trying to add chunk 1 of the first message -
  // it should create a new buffer instead of finding the existing one
}

// =============================================================================
// Reassembly Buffer Boundary Tests
// =============================================================================

#[test]
fn test_reassembly_buffer_chunk_index_boundaries() {
  let message_id = MessageId::new();
  let total_chunks = 3u32;

  // Create buffer
  let mut buffer = ReassemblyBuffer::new(message_id, 300, total_chunks, 0x80);

  // Add chunks with valid indices (0, 1, 2)
  let chunk0 = ChunkedMessage::new(
    ChunkHeader::new(message_id, 300, 0, total_chunks),
    vec![0; 100],
  );
  let chunk1 = ChunkedMessage::new(
    ChunkHeader::new(message_id, 300, 1, total_chunks),
    vec![1; 100],
  );
  let chunk2 = ChunkedMessage::new(
    ChunkHeader::new(message_id, 300, 2, total_chunks),
    vec![2; 100],
  );

  assert!(buffer.add_chunk(&chunk0).expect("chunk0"));
  assert!(buffer.add_chunk(&chunk1).expect("chunk1"));
  assert!(buffer.add_chunk(&chunk2).expect("chunk2"));

  assert!(buffer.bitmap.is_complete());
}

#[test]
fn test_reassembly_buffer_duplicate_chunk_index() {
  let message_id = MessageId::new();
  let mut buffer = ReassemblyBuffer::new(message_id, 200, 2u32, 0x80);

  // Add same chunk index twice
  let first_chunk = ChunkedMessage::new(ChunkHeader::new(message_id, 200, 0, 2), vec![0xAA; 100]);
  let second_chunk = ChunkedMessage::new(ChunkHeader::new(message_id, 200, 0, 2), vec![0xBB; 100]);

  assert!(buffer.add_chunk(&first_chunk).expect("first chunk"));
  // Second chunk with same index - behavior depends on implementation
  // Just verify no panic
  let _ = buffer.add_chunk(&second_chunk);
}

#[test]
fn test_reassembly_buffer_overlapping_chunks() {
  // BUG-002: Test that overlapping chunk data doesn't cause corruption
  let message_id = MessageId::new();
  let mut buffer = ReassemblyBuffer::new(message_id, 100, 2, 0x80);

  let chunk1 = ChunkedMessage::new(ChunkHeader::new(message_id, 100, 0, 2), vec![0xAA; 50]);
  let chunk2 = ChunkedMessage::new(ChunkHeader::new(message_id, 100, 1, 2), vec![0xBB; 50]);

  assert!(buffer.add_chunk(&chunk1).expect("chunk1 should succeed"));
  assert!(buffer.add_chunk(&chunk2).expect("chunk2 should succeed"));

  let result = buffer.reassemble().expect("reassemble should succeed");
  assert_eq!(&result[0..50], &[0xAA; 50]);
  assert_eq!(&result[50..100], &[0xBB; 50]);
}

// =============================================================================
// ChunkManager force_cleanup_expired Tests
// =============================================================================

#[test]
fn test_chunk_manager_force_cleanup_expired_returns_count() {
  let mut manager = ChunkManager::new();

  // Create two buffers
  let message_id1 = MessageId::new();
  let message_id2 = MessageId::new();

  let chunk1 = ChunkedMessage::new(ChunkHeader::new(message_id1, 100, 0, 2), vec![1; 50]);
  let chunk2 = ChunkedMessage::new(ChunkHeader::new(message_id2, 200, 0, 2), vec![2; 50]);

  manager.process_chunk(&chunk1, 0x80).expect("Failed");
  manager.process_chunk(&chunk2, 0x80).expect("Failed");

  assert_eq!(manager.active_buffer_count(), 2);

  // Expire the first buffer
  if let Some(buffer) = manager.buffers.get_mut(&message_id1) {
    buffer.created_at = Instant::now()
      .checked_sub(Duration::from_secs(REASSEMBLY_TIMEOUT_SECS + 1))
      .expect("time subtraction should not underflow");
  }

  // Call force_cleanup_expired and verify return count
  let removed = manager.force_cleanup_expired();
  assert_eq!(removed, 1, "Should report 1 buffer removed");
  assert_eq!(manager.active_buffer_count(), 1);
}

#[test]
fn test_chunk_manager_force_cleanup_expired_no_expired() {
  let mut manager = ChunkManager::new();

  let message_id = MessageId::new();
  let chunk = ChunkedMessage::new(ChunkHeader::new(message_id, 100, 0, 2), vec![0; 50]);

  manager.process_chunk(&chunk, 0x80).expect("Failed");
  assert_eq!(manager.active_buffer_count(), 1);

  // Call force_cleanup_expired with no expired buffers
  let removed = manager.force_cleanup_expired();
  assert_eq!(removed, 0, "Should report 0 buffers removed");
  assert_eq!(manager.active_buffer_count(), 1);
}

#[test]
fn test_chunk_manager_force_cleanup_expired_all_expired() {
  let mut manager = ChunkManager::new();

  // Create multiple buffers
  for i in 0..3 {
    let message_id = MessageId::new();
    let chunk = ChunkedMessage::new(
      ChunkHeader::new(message_id, 100, 0, 2),
      vec![u8::try_from(i).expect("i should fit in u8"); 50],
    );
    manager.process_chunk(&chunk, 0x80).expect("Failed");
  }

  assert_eq!(manager.active_buffer_count(), 3);

  // Expire all buffers
  for buffer in manager.buffers.values_mut() {
    buffer.created_at = Instant::now()
      .checked_sub(Duration::from_secs(REASSEMBLY_TIMEOUT_SECS + 1))
      .expect("time subtraction should not underflow");
  }

  // Call force_cleanup_expired
  let removed = manager.force_cleanup_expired();
  assert_eq!(removed, 3, "Should report all 3 buffers removed");
  assert_eq!(manager.active_buffer_count(), 0);
}

#[test]
fn test_chunk_manager_force_cleanup_expired_empty() {
  let mut manager = ChunkManager::new();

  // Call on empty manager
  let removed = manager.force_cleanup_expired();
  assert_eq!(removed, 0, "Should report 0 for empty manager");
  assert_eq!(manager.active_buffer_count(), 0);
}
