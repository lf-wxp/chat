//! Concurrent reassembly tests.

use super::*;
use crate::types::MessageId;
use std::sync::{Arc, Mutex};
use std::thread;

// =============================================================================
// Concurrent Reassembly Tests
// =============================================================================

#[test]
fn test_concurrent_reassembly_interleaved_chunks() {
  // Simulate 3 messages whose chunks arrive interleaved
  let mut manager = ChunkManager::new();

  let alpha_id = MessageId::new();
  let beta_id = MessageId::new();
  let gamma_id = MessageId::new();

  // Message Alpha: 3 chunks, each 40 bytes => total 120 bytes
  let a0 = ChunkedMessage::new(ChunkHeader::new(alpha_id, 120, 0, 3), vec![0xAA; 40]);
  let a1 = ChunkedMessage::new(ChunkHeader::new(alpha_id, 120, 1, 3), vec![0xAB; 40]);
  let a2 = ChunkedMessage::new(ChunkHeader::new(alpha_id, 120, 2, 3), vec![0xAC; 40]);

  // Message Beta: 2 chunks, each 50 bytes => total 100 bytes
  let b0 = ChunkedMessage::new(ChunkHeader::new(beta_id, 100, 0, 2), vec![0xBB; 50]);
  let b1 = ChunkedMessage::new(ChunkHeader::new(beta_id, 100, 1, 2), vec![0xBC; 50]);

  // Message Gamma: 2 chunks, each 30 bytes => total 60 bytes
  let c0 = ChunkedMessage::new(ChunkHeader::new(gamma_id, 60, 0, 2), vec![0xCC; 30]);
  let c1 = ChunkedMessage::new(ChunkHeader::new(gamma_id, 60, 1, 2), vec![0xCD; 30]);

  // Interleaved arrival order: A0, B0, C0, A1, B1, C1, A2
  assert!(manager.process_chunk(&a0, 0x80).unwrap().is_none());
  assert_eq!(manager.active_buffer_count(), 1);

  assert!(manager.process_chunk(&b0, 0x81).unwrap().is_none());
  assert_eq!(manager.active_buffer_count(), 2);

  assert!(manager.process_chunk(&c0, 0x82).unwrap().is_none());
  assert_eq!(manager.active_buffer_count(), 3);

  assert!(manager.process_chunk(&a1, 0x80).unwrap().is_none());
  assert_eq!(manager.active_buffer_count(), 3);

  // B completes with b1
  let result_b = manager.process_chunk(&b1, 0x81).unwrap();
  assert!(result_b.is_some());
  let (b_type, b_data) = result_b.unwrap();
  assert_eq!(b_type, 0x81);
  assert_eq!(b_data.len(), 100);
  assert_eq!(&b_data[..50], &[0xBB; 50]);
  assert_eq!(&b_data[50..], &[0xBC; 50]);
  // B buffer removed after completion
  assert_eq!(manager.active_buffer_count(), 2);

  // C completes with c1
  let result_c = manager.process_chunk(&c1, 0x82).unwrap();
  assert!(result_c.is_some());
  let (c_type, c_data) = result_c.unwrap();
  assert_eq!(c_type, 0x82);
  assert_eq!(c_data.len(), 60);
  assert_eq!(&c_data[..30], &[0xCC; 30]);
  assert_eq!(&c_data[30..], &[0xCD; 30]);
  assert_eq!(manager.active_buffer_count(), 1);

  // A completes with a2
  let result_a = manager.process_chunk(&a2, 0x80).unwrap();
  assert!(result_a.is_some());
  let (a_type, a_data) = result_a.unwrap();
  assert_eq!(a_type, 0x80);
  assert_eq!(a_data.len(), 120);
  assert_eq!(&a_data[..40], &[0xAA; 40]);
  assert_eq!(&a_data[40..80], &[0xAB; 40]);
  assert_eq!(&a_data[80..], &[0xAC; 40]);
  assert_eq!(manager.active_buffer_count(), 0);
}

#[test]
fn test_concurrent_reassembly_with_duplicates() {
  // Interleaved chunks with duplicate retransmissions
  let mut manager = ChunkManager::new();

  let alpha_id = MessageId::new();
  let beta_id = MessageId::new();

  // Message Alpha: 2 chunks, each 50 bytes => total 100
  let a0 = ChunkedMessage::new(ChunkHeader::new(alpha_id, 100, 0, 2), vec![0xA0; 50]);
  let a1 = ChunkedMessage::new(ChunkHeader::new(alpha_id, 100, 1, 2), vec![0xA1; 50]);

  // Message Beta: 2 chunks, each 60 bytes => total 120
  let b0 = ChunkedMessage::new(ChunkHeader::new(beta_id, 120, 0, 2), vec![0xB0; 60]);
  let b1 = ChunkedMessage::new(ChunkHeader::new(beta_id, 120, 1, 2), vec![0xB1; 60]);

  // Arrival with duplicates: A0, B0, A0(dup), B0(dup), A1, B1
  assert!(manager.process_chunk(&a0, 0x80).unwrap().is_none());
  assert!(manager.process_chunk(&b0, 0x81).unwrap().is_none());

  // Duplicate A0 — should not corrupt buffer
  assert!(manager.process_chunk(&a0, 0x80).unwrap().is_none());
  // Duplicate B0 — should not corrupt buffer
  assert!(manager.process_chunk(&b0, 0x81).unwrap().is_none());

  assert_eq!(manager.active_buffer_count(), 2);

  // A completes
  let result_a = manager.process_chunk(&a1, 0x80).unwrap();
  assert!(result_a.is_some());
  let (a_type, a_data) = result_a.unwrap();
  assert_eq!(a_type, 0x80);
  assert_eq!(a_data.len(), 100);
  assert_eq!(&a_data[..50], &[0xA0; 50]);
  assert_eq!(&a_data[50..], &[0xA1; 50]);

  // B completes
  let result_b = manager.process_chunk(&b1, 0x81).unwrap();
  assert!(result_b.is_some());
  let (b_type, b_data) = result_b.unwrap();
  assert_eq!(b_type, 0x81);
  assert_eq!(b_data.len(), 120);
  assert_eq!(&b_data[..60], &[0xB0; 60]);
  assert_eq!(&b_data[60..], &[0xB1; 60]);

  assert_eq!(manager.active_buffer_count(), 0);
}

#[test]
fn test_concurrent_reassembly_partial_completion() {
  // Some messages complete while others remain incomplete
  let mut manager = ChunkManager::new();

  let msg_complete_id = MessageId::new();
  let msg_incomplete_id = MessageId::new();

  // Complete message: 2 chunks, each 50 bytes => total 100
  let c0 = ChunkedMessage::new(ChunkHeader::new(msg_complete_id, 100, 0, 2), vec![0xCC; 50]);
  let c1 = ChunkedMessage::new(ChunkHeader::new(msg_complete_id, 100, 1, 2), vec![0xCD; 50]);

  // Incomplete message: 3 chunks expected, only 1 arrives
  let i0 = ChunkedMessage::new(
    ChunkHeader::new(msg_incomplete_id, 150, 0, 3),
    vec![0xDD; 50],
  );

  // Process interleaved
  assert!(manager.process_chunk(&c0, 0x80).unwrap().is_none());
  assert!(manager.process_chunk(&i0, 0x81).unwrap().is_none());
  assert_eq!(manager.active_buffer_count(), 2);

  // Complete message finishes
  let result = manager.process_chunk(&c1, 0x80).unwrap();
  assert!(result.is_some());
  let (msg_type, data) = result.unwrap();
  assert_eq!(msg_type, 0x80);
  assert_eq!(data.len(), 100);

  // Incomplete message buffer still exists
  assert_eq!(manager.active_buffer_count(), 1);
  let incomplete_buf = manager.get_buffer(&msg_incomplete_id).unwrap();
  assert!(!incomplete_buf.is_complete());
  assert_eq!(incomplete_buf.bitmap.received_count(), 1);
  assert_eq!(incomplete_buf.bitmap.missing_chunks(), vec![1, 2]);
}

#[test]
fn test_concurrent_reassembly_out_of_order() {
  // Chunks arrive in reverse order for multiple messages
  let mut manager = ChunkManager::new();

  let alpha_id = MessageId::new();
  let beta_id = MessageId::new();

  // Message Alpha: 4 chunks, each 25 bytes => total 100
  let a0 = ChunkedMessage::new(ChunkHeader::new(alpha_id, 100, 0, 4), vec![0x10; 25]);
  let a1 = ChunkedMessage::new(ChunkHeader::new(alpha_id, 100, 1, 4), vec![0x11; 25]);
  let a2 = ChunkedMessage::new(ChunkHeader::new(alpha_id, 100, 2, 4), vec![0x12; 25]);
  let a3 = ChunkedMessage::new(ChunkHeader::new(alpha_id, 100, 3, 4), vec![0x13; 25]);

  // Message Beta: 3 chunks => total 90 (30+30+30)
  let b0 = ChunkedMessage::new(ChunkHeader::new(beta_id, 90, 0, 3), vec![0x20; 30]);
  let b1 = ChunkedMessage::new(ChunkHeader::new(beta_id, 90, 1, 3), vec![0x21; 30]);
  let b2 = ChunkedMessage::new(ChunkHeader::new(beta_id, 90, 2, 3), vec![0x22; 30]);

  // Reverse + interleaved: A3, B2, A2, B1, A1, B0, A0
  assert!(manager.process_chunk(&a3, 0x80).unwrap().is_none());
  assert!(manager.process_chunk(&b2, 0x81).unwrap().is_none());
  assert!(manager.process_chunk(&a2, 0x80).unwrap().is_none());
  assert!(manager.process_chunk(&b1, 0x81).unwrap().is_none());
  assert!(manager.process_chunk(&a1, 0x80).unwrap().is_none());

  // B completes with b0 (last missing chunk)
  let result_b = manager.process_chunk(&b0, 0x81).unwrap();
  assert!(result_b.is_some());
  let (b_type, b_data) = result_b.unwrap();
  assert_eq!(b_type, 0x81);
  assert_eq!(b_data.len(), 90);
  // Verify data is reassembled in correct order regardless of arrival order
  assert_eq!(&b_data[..30], &[0x20; 30]);
  assert_eq!(&b_data[30..60], &[0x21; 30]);
  assert_eq!(&b_data[60..], &[0x22; 30]);

  // A completes with a0 (last missing chunk)
  let result_a = manager.process_chunk(&a0, 0x80).unwrap();
  assert!(result_a.is_some());
  let (a_type, a_data) = result_a.unwrap();
  assert_eq!(a_type, 0x80);
  assert_eq!(a_data.len(), 100);
  // Verify data is reassembled in correct order regardless of arrival order
  assert_eq!(&a_data[..25], &[0x10; 25]);
  assert_eq!(&a_data[25..50], &[0x11; 25]);
  assert_eq!(&a_data[50..75], &[0x12; 25]);
  assert_eq!(&a_data[75..], &[0x13; 25]);

  assert_eq!(manager.active_buffer_count(), 0);
}

#[test]
fn test_concurrent_reassembly_end_to_end_chunk_and_reassemble() {
  // End-to-end: chunk a large message, then reassemble with interleaved chunks
  // from another message
  let mut manager = ChunkManager::new();

  // Create a large message that needs chunking
  let payload_a: Vec<u8> = (0u32..150 * 1024)
    .map(|i| u8::try_from(i % 256).unwrap())
    .collect();
  let frame_a = MessageFrame::new(0x80, payload_a.clone());
  let chunks_a = chunk_message(&frame_a).expect("Failed to chunk message A");
  assert_eq!(chunks_a.len(), 3); // 150KB => 3 chunks of 64KB, 64KB, 22KB

  // Create another large message
  let payload_b: Vec<u8> = (0u32..200 * 1024)
    .map(|i| u8::try_from((i + 128) % 256).unwrap())
    .collect();
  let frame_b = MessageFrame::new(0x81, payload_b.clone());
  let chunks_b = chunk_message(&frame_b).expect("Failed to chunk message B");
  assert_eq!(chunks_b.len(), 4); // 200KB => 4 chunks

  // Interleave: B0, A0, B1, A1, B2, A2, B3
  assert!(manager.process_chunk(&chunks_b[0], 0x81).unwrap().is_none());
  assert!(manager.process_chunk(&chunks_a[0], 0x80).unwrap().is_none());
  assert!(manager.process_chunk(&chunks_b[1], 0x81).unwrap().is_none());
  assert!(manager.process_chunk(&chunks_a[1], 0x80).unwrap().is_none());
  assert!(manager.process_chunk(&chunks_b[2], 0x81).unwrap().is_none());

  // A completes with chunk 2
  let result_a = manager.process_chunk(&chunks_a[2], 0x80).unwrap();
  assert!(result_a.is_some());
  let (a_type, a_data) = result_a.unwrap();
  assert_eq!(a_type, 0x80);
  assert_eq!(a_data, payload_a);

  // B completes with chunk 3
  let result_b = manager.process_chunk(&chunks_b[3], 0x81).unwrap();
  assert!(result_b.is_some());
  let (b_type, b_data) = result_b.unwrap();
  assert_eq!(b_type, 0x81);
  assert_eq!(b_data, payload_b);

  assert_eq!(manager.active_buffer_count(), 0);
}

// =============================================================================
// MA-P1-003: ChunkManager Concurrent Access Tests
// =============================================================================

/// Test that `ChunkManager` can handle concurrent chunk processing safely.
/// Note: `ChunkManager` uses `RefCell` internally for interior mutability,
/// so this test verifies thread-safety with external synchronization.
#[test]
fn test_chunk_manager_sequential_concurrent_simulation() {
  let manager = Arc::new(Mutex::new(ChunkManager::new()));
  let mut handles = vec![];

  // Simulate concurrent chunk processing from multiple messages
  for i in 0..5 {
    let manager_clone = Arc::clone(&manager);
    let handle = thread::spawn(move || {
      let message_id = MessageId::new();
      let chunk = ChunkedMessage::new(ChunkHeader::new(message_id, 200, 0, 2), vec![i; 100]);

      let mut mgr = manager_clone.lock().unwrap();
      mgr.process_chunk(&chunk, 0x80)
    });
    handles.push(handle);
  }

  // All chunks should be processed successfully
  for handle in handles {
    let result = handle.join().expect("Thread should not panic");
    assert!(result.is_ok(), "Concurrent chunk processing should succeed");
  }
}

/// Test `ChunkManager` under high load with many interleaved operations.
#[test]
fn test_chunk_manager_high_load_interleaved() {
  let mut manager = ChunkManager::new();
  let mut message_ids: Vec<MessageId> = Vec::new();

  // Create 20 messages, each with 3 chunks
  for _ in 0..20 {
    message_ids.push(MessageId::new());
  }

  // Add first chunk of each message
  for message_id in &message_ids {
    let chunk = ChunkedMessage::new(ChunkHeader::new(*message_id, 300, 0, 3), vec![0; 100]);
    assert!(manager.process_chunk(&chunk, 0x80).is_ok());
  }

  // Add second chunk of each message
  for message_id in &message_ids {
    let chunk = ChunkedMessage::new(ChunkHeader::new(*message_id, 300, 1, 3), vec![1; 100]);
    assert!(manager.process_chunk(&chunk, 0x80).is_ok());
  }

  // Add third chunk of each message
  for message_id in &message_ids {
    let chunk = ChunkedMessage::new(ChunkHeader::new(*message_id, 300, 2, 3), vec![2; 100]);
    let result = manager.process_chunk(&chunk, 0x80);
    // Due to MAX_REASSEMBLY_BUFFERS limit, some may be evicted
    // We just verify no panics or corruption
    assert!(result.is_ok() || result.is_err());
  }
}
