//! Integration tests for the file-transfer subsystem.
//!
//! These tests exercise end-to-end flows that span multiple modules
//! (send → receive → hash verification) without requiring a real
//! WebRTC DataChannel. They complement the unit tests in `tests.rs`
//! by verifying that the sender-side chunking, receiver-side
//! reassembly, and SHA-256 hash verification compose correctly.

use super::hash;
use super::receive::IncomingTransfer;
use super::send::OutgoingTransfer;
use super::types::{FileInfo, TransferDirection, TransferProgress, TransferStatus};
use leptos::prelude::{GetUntracked, RwSignal};
use message::{MessageId, TransferId, UserId};

fn make_info(total_chunks: u32, size: u64, chunk_size: u32) -> FileInfo {
  FileInfo {
    message_id: MessageId::new(),
    transfer_id: TransferId::new(),
    filename: "integration-test.bin".into(),
    size,
    mime_type: "application/octet-stream".into(),
    file_hash: [0u8; 32],
    total_chunks,
    chunk_size,
    room_id: None,
  }
}

/// Simulate a full send → receive → reassemble cycle with hash
/// verification. This is the primary integration test: the sender
/// slices a byte buffer into chunks, the receiver records them in
/// random order, and the reassembled output matches the original.
#[test]
fn full_send_receive_cycle_with_hash_verification() {
  let chunk_size: usize = 64;
  let original = vec![0xAB_u8; 256]; // 4 chunks of 64 bytes
  let total_chunks = 4u32;
  let file_hash = hash::sha256_sync(&original);

  // --- Sender side ---
  let info = FileInfo {
    file_hash,
    ..make_info(total_chunks, original.len() as u64, chunk_size as u32)
  };
  let peers = vec![UserId::from(1u64)];
  let progress = RwSignal::new(TransferProgress::new(info.size, info.total_chunks));
  let status = RwSignal::new(TransferStatus::InProgress);
  let tx = OutgoingTransfer {
    info: info.clone(),
    bytes: original.clone(),
    object_url: String::new(),
    thumbnail_url: RwSignal::new(None),
    targets: peers.clone(),
    progress,
    status,
    direction: TransferDirection::Outgoing,
  };

  // Slice into chunks the same way dispatch.rs does.
  let chunks: Vec<(u32, Vec<u8>)> = (0..total_chunks)
    .map(|idx| {
      let start = (idx as usize) * chunk_size;
      let end = (start + chunk_size).min(tx.bytes.len());
      (idx, tx.bytes[start..end].to_vec())
    })
    .collect();
  assert_eq!(chunks.len(), 4);

  // --- Receiver side ---
  let mut rx = IncomingTransfer::new(info.clone(), peers[0].clone());

  // Deliver chunks out of order (2, 0, 3, 1) to simulate network
  // reordering.
  let order = [2u32, 0, 3, 1];
  for &idx in &order {
    let (_, ref data) = chunks[idx as usize];
    assert!(rx.record_chunk(idx, data.clone(), None).unwrap());
  }

  // Verify reassembly.
  assert!(rx.is_complete());
  let reassembled = rx.reassemble().unwrap();
  assert_eq!(reassembled, original);

  // Verify SHA-256 matches.
  let received_hash = hash::sha256_sync(&reassembled);
  assert_eq!(
    received_hash, file_hash,
    "file hash must match after reassembly"
  );
}

/// Simulate a hash-mismatch scenario: the sender announces a hash,
/// the receiver reassembles the file, and the computed hash does
/// NOT match the announced hash (simulating data corruption).
#[test]
fn hash_mismatch_detected_after_reassembly() {
  let chunk_size: usize = 64;
  let original = [0xCD_u8; 128]; // 2 chunks
  let file_hash = hash::sha256_sync(&original);

  // Announce the correct hash, but deliver corrupted data.
  let corrupted = [0xEF_u8; 128];
  let info = FileInfo {
    file_hash, // announced hash was computed from `original`
    ..make_info(2, corrupted.len() as u64, chunk_size as u32)
  };

  let mut rx = IncomingTransfer::new(info, UserId::from(99u64));

  // Deliver corrupted chunks.
  for idx in 0..2u32 {
    let start = (idx as usize) * chunk_size;
    let end = (start + chunk_size).min(corrupted.len());
    rx.record_chunk(idx, corrupted[start..end].to_vec(), None)
      .unwrap();
  }

  assert!(rx.is_complete());
  let reassembled = rx.reassemble().unwrap();

  // Compute the hash of what we actually received.
  let actual_hash = hash::sha256_sync(&reassembled);
  assert_ne!(actual_hash, file_hash, "hash mismatch must be detectable");
}

/// Simulate disconnect-resume: the sender sends 3 of 5 chunks,
/// the connection drops, then the receiver reports missing chunks
/// and the sender re-sends them.
#[test]
fn disconnect_resume_with_missing_chunks() {
  let chunk_size: usize = 32;
  let original = vec![0x77_u8; 160]; // 5 chunks
  let file_hash = hash::sha256_sync(&original);
  let info = FileInfo {
    file_hash,
    ..make_info(5, original.len() as u64, chunk_size as u32)
  };

  // --- Initial round: receive 3 of 5 chunks ---
  let mut rx = IncomingTransfer::new(info.clone(), UserId::from(42u64));

  // Only chunks 0, 2, 4 arrive before disconnect.
  for &idx in &[0u32, 2, 4] {
    let start = (idx as usize) * chunk_size;
    let end = (start + chunk_size).min(original.len());
    rx.record_chunk(idx, original[start..end].to_vec(), None)
      .unwrap();
  }

  // Verify gaps.
  let missing = rx.missing_chunks();
  assert_eq!(missing, vec![1, 3]);
  assert!(!rx.is_complete());

  // --- Resume round: sender re-sends missing chunks ---
  for &idx in &missing {
    let start = (idx as usize) * chunk_size;
    let end = (start + chunk_size).min(original.len());
    rx.record_chunk(idx, original[start..end].to_vec(), None)
      .unwrap();
  }

  assert!(rx.is_complete());
  let reassembled = rx.reassemble().unwrap();
  assert_eq!(reassembled, original);

  // Final hash matches.
  let received_hash = hash::sha256_sync(&reassembled);
  assert_eq!(received_hash, file_hash);
}

/// Simulate the full re-receive flow after a hash mismatch:
/// the user clicks "Re-receive", the buffer is reset, and the
/// file is received again from scratch (P1-1 fix).
#[test]
fn hash_mismatch_triggers_full_rereceive() {
  let chunk_size: usize = 48;
  let original = vec![0x11_u8; 96]; // 2 chunks
  let file_hash = hash::sha256_sync(&original);

  let info = FileInfo {
    file_hash,
    ..make_info(2, original.len() as u64, chunk_size as u32)
  };

  let mut rx = IncomingTransfer::new(info.clone(), UserId::from(7u64));

  // First round: receive corrupted data.
  let corrupted = vec![0xFF_u8; 48];
  rx.record_chunk(0, corrupted.clone(), None).unwrap();
  rx.record_chunk(1, corrupted.clone(), None).unwrap();

  assert!(rx.is_complete());
  let reassembled = rx.reassemble().unwrap();
  let actual_hash = hash::sha256_sync(&reassembled);
  assert_ne!(actual_hash, file_hash, "corrupted data must not match");

  // User triggers "Re-receive" (P1-1 fix): reset the buffer.
  rx.reset_for_resume();
  assert!(!rx.is_complete());
  assert_eq!(rx.missing_chunks(), vec![0, 1]);

  // Second round: receive the correct data.
  for idx in 0..2u32 {
    let start = (idx as usize) * chunk_size;
    let end = (start + chunk_size).min(original.len());
    rx.record_chunk(idx, original[start..end].to_vec(), None)
      .unwrap();
  }

  assert!(rx.is_complete());
  let reassembled2 = rx.reassemble().unwrap();
  assert_eq!(reassembled2, original);

  let final_hash = hash::sha256_sync(&reassembled2);
  assert_eq!(final_hash, file_hash, "re-received file must hash-match");
}

/// Simulate multi-peer serial dispatch: the sender transmits
/// the same file to 3 peers. Verify that progress aggregation
/// across all peers is correct.
#[test]
fn multi_peer_serial_dispatch_progress() {
  let chunk_size: usize = 64;
  let original = vec![0x42_u8; 256]; // 4 chunks
  let file_hash = hash::sha256_sync(&original);

  let info = FileInfo {
    file_hash,
    ..make_info(4, original.len() as u64, chunk_size as u32)
  };

  let peers = vec![UserId::from(1u64), UserId::from(2u64), UserId::from(3u64)];

  let progress = RwSignal::new(TransferProgress::new(info.size, info.total_chunks));
  let status = RwSignal::new(TransferStatus::InProgress);
  let tx = OutgoingTransfer {
    info,
    bytes: original,
    object_url: String::new(),
    thumbnail_url: RwSignal::new(None),
    targets: peers.clone(),
    progress,
    status,
    direction: TransferDirection::Outgoing,
  };

  // Simulate sending 2 chunks to peer 1.
  tx.advance(&peers[0], chunk_size as u64);
  tx.advance(&peers[0], chunk_size as u64);
  tx.record_throughput(500);

  // Simulate sending 4 chunks to peer 2.
  for _ in 0..4 {
    tx.advance(&peers[1], chunk_size as u64);
  }
  tx.record_throughput(1000);

  // Peer 3 not started yet.

  let p = tx.progress.get_untracked();

  // Total transferred = 2*64 + 4*64 = 384 bytes.
  assert_eq!(p.transferred_bytes, 384);
  assert_eq!(p.chunks_done, 6);

  // 2 peers have been advanced so total work = 256 * 2 = 512.
  // Percent = 384 / 512 * 100 = 75.
  assert_eq!(p.percent(), 75);

  // Throughput should be non-zero.
  assert!(p.bytes_per_sec > 0);
}

/// Per-chunk hash validation during an end-to-end transfer:
/// all chunks carry their SHA-256, and one chunk is corrupted.
/// The receiver drops the corrupted chunk, and after a resume
/// round the file is correctly reassembled.
#[test]
fn per_chunk_hash_validation_during_transfer() {
  let chunk_size: usize = 32;
  let original = vec![0x55_u8; 128]; // 4 chunks
  let file_hash = hash::sha256_sync(&original);

  let info = FileInfo {
    file_hash,
    ..make_info(4, original.len() as u64, chunk_size as u32)
  };

  let mut rx = IncomingTransfer::new(info, UserId::from(10u64));

  // Send chunk 0 with correct hash.
  let data0 = original[0..32].to_vec();
  let hash0 = hash::sha256_sync(&data0);
  assert!(rx.record_chunk(0, data0, Some(&hash0)).unwrap());

  // Send chunk 1 with TAMPERED data but the CORRECT hash.
  let tampered = vec![0xAA_u8; 32];
  let correct_hash1 = hash::sha256_sync(&original[32..64]);
  let err = rx
    .record_chunk(1, tampered, Some(&correct_hash1))
    .unwrap_err();
  assert!(err.contains("hash mismatch"));

  // Chunk 1 is still missing.
  assert_eq!(rx.missing_chunks(), vec![1, 2, 3]);

  // Send chunk 2 with correct hash.
  let data2 = original[64..96].to_vec();
  let hash2 = hash::sha256_sync(&data2);
  assert!(rx.record_chunk(2, data2, Some(&hash2)).unwrap());

  // Resume: send chunk 1 with correct data and hash.
  let data1 = original[32..64].to_vec();
  assert!(rx.record_chunk(1, data1, Some(&correct_hash1)).unwrap());

  // Send chunk 3.
  let data3 = original[96..128].to_vec();
  let hash3 = hash::sha256_sync(&data3);
  assert!(rx.record_chunk(3, data3, Some(&hash3)).unwrap());

  assert!(rx.is_complete());
  let reassembled = rx.reassemble().unwrap();
  assert_eq!(reassembled, original);

  let final_hash = hash::sha256_sync(&reassembled);
  assert_eq!(final_hash, file_hash);
}

/// Verify that `IncomingTransfer::hash()` produces the same
/// result as `hash::sha256_sync()` on the reassembled bytes.
#[test]
fn incoming_transfer_hash_matches_reassembled() {
  let chunk_size: usize = 64;
  let original = [0x33_u8; 192]; // 3 chunks

  let info = make_info(3, original.len() as u64, chunk_size as u32);
  let mut rx = IncomingTransfer::new(info, UserId::from(20u64));

  for idx in 0..3u32 {
    let start = (idx as usize) * chunk_size;
    let end = (start + chunk_size).min(original.len());
    rx.record_chunk(idx, original[start..end].to_vec(), None)
      .unwrap();
  }

  assert!(rx.is_complete());
  let via_method = rx.hash();
  let via_reassemble = hash::sha256_sync(&rx.reassemble().unwrap());
  assert_eq!(via_method, via_reassemble);
}

/// Verify that a sender can rebuild chunks for resume identically
/// to the original dispatch, across the full set of chunk indices.
#[test]
fn sender_resume_chunk_slicing_matches_original() {
  let chunk_size: usize = 64;
  let original = [0xBB_u8; 200]; // 4 chunks: 64+64+64+8

  // Build chunks the same way dispatch.rs does.
  let total_chunks = (original.len().div_ceil(chunk_size)) as u32;
  let chunks: Vec<Vec<u8>> = (0..total_chunks)
    .map(|idx| {
      let start = (idx as usize) * chunk_size;
      let end = (start + chunk_size).min(original.len());
      original[start..end].to_vec()
    })
    .collect();

  assert_eq!(chunks.len(), 4);
  assert_eq!(chunks[0].len(), 64);
  assert_eq!(chunks[3].len(), 8); // last chunk is short

  // Rebuild from original bytes for a subset (simulating resume).
  let resume_indices = vec![1, 3];
  for &idx in &resume_indices {
    let start = (idx as usize) * chunk_size;
    let end = (start + chunk_size).min(original.len());
    let rebuilt = &original[start..end];
    assert_eq!(rebuilt, chunks[idx as usize].as_slice());
  }
}
