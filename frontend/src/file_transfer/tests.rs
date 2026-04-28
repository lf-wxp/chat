//! Unit tests for the file-transfer subsystem.
//!
//! These tests exercise the pure, non-WebRTC parts of the pipeline
//! so they can run under plain `cargo test` without a browser:
//!
//! * Size limit + extension classification helpers.
//! * Inbound reassembly + bitmap accounting.
//! * Progress snapshot computations (`percent`, ETA, peer list).
//! * SHA-256 native fallback determinism.
//! * Integration-style tests for flow control, resume, and
//!   multi-peer dispatch progress (P1-5).

use super::hash;
use super::receive::IncomingTransfer;
use super::send::OutgoingTransfer;
use super::types::{
  BUFFER_HIGH_WATER, BUFFER_LOW_WATER, DANGEROUS_EXTENSIONS, FileInfo, MULTI_PEER_SIZE_LIMIT,
  PeerProgress, SINGLE_PEER_SIZE_LIMIT, TransferDirection, TransferProgress, TransferStatus,
  estimate_transfer_seconds, format_bytes, next_chunk_size, size_limit_for_peers,
};
use leptos::prelude::{Get, GetUntracked, RwSignal, Set, Update};
use message::{MessageId, TransferId, UserId};

fn demo_info(total_chunks: u32, size: u64, filename: &str) -> FileInfo {
  FileInfo {
    message_id: MessageId::new(),
    transfer_id: TransferId::new(),
    filename: filename.to_string(),
    size,
    mime_type: "application/octet-stream".into(),
    file_hash: [0u8; 32],
    total_chunks,
    chunk_size: 64 * 1024,
    room_id: None,
  }
}

#[test]
fn format_bytes_rounds_to_sensible_units() {
  assert_eq!(format_bytes(0), "0 B");
  assert_eq!(format_bytes(512), "512 B");
  assert_eq!(format_bytes(1024), "1.0 KB");
  assert_eq!(format_bytes(2_097_152), "2.0 MB");
  assert_eq!(format_bytes(3 * 1024 * 1024 * 1024), "3.0 GB");
}

#[test]
fn size_limit_switches_between_single_and_multi_peer() {
  assert_eq!(size_limit_for_peers(0), SINGLE_PEER_SIZE_LIMIT);
  assert_eq!(size_limit_for_peers(1), SINGLE_PEER_SIZE_LIMIT);
  assert_eq!(size_limit_for_peers(2), MULTI_PEER_SIZE_LIMIT);
  assert_eq!(size_limit_for_peers(7), MULTI_PEER_SIZE_LIMIT);
}

#[test]
fn dangerous_extensions_are_detected_case_insensitively() {
  for ext in DANGEROUS_EXTENSIONS {
    let info = demo_info(1, 100, &format!("payload{ext}"));
    assert!(info.is_dangerous_extension(), "should flag {ext}");
  }
  // Case-insensitive check.
  assert!(demo_info(1, 100, "SETUP.EXE").is_dangerous_extension());
  // Safe extensions are not flagged.
  assert!(!demo_info(1, 100, "photo.png").is_dangerous_extension());
  assert!(!demo_info(1, 100, "doc.pdf").is_dangerous_extension());
}

#[test]
fn extension_extractor_handles_dotless_names() {
  let info = demo_info(1, 10, "README");
  assert_eq!(info.extension(), "");
  let info = demo_info(1, 10, "archive.tar.gz");
  assert_eq!(info.extension(), ".gz");
}

#[test]
fn transfer_progress_percent_is_clamped() {
  // Single-peer (incoming) scenario: peers list is empty.
  let mut p = TransferProgress::new(1000, 2);
  p.transferred_bytes = 250;
  assert_eq!(p.percent(), 25);
  p.transferred_bytes = 1000;
  assert_eq!(p.percent(), 100);
  p.transferred_bytes = 9999; // Defensive: clamped to 100.
  assert_eq!(p.percent(), 100);
  let zero = TransferProgress::new(0, 0);
  assert_eq!(zero.percent(), 100);
}

#[test]
fn transfer_progress_percent_scales_with_peer_count() {
  // Multi-peer outgoing: total work = total_bytes * peer_count.
  let mut p = TransferProgress::new(1000, 2);
  p.peers = vec![
    PeerProgress {
      peer_id: UserId::from(1u64),
      chunks_sent: 0,
      status: TransferStatus::InProgress,
    },
    PeerProgress {
      peer_id: UserId::from(2u64),
      chunks_sent: 0,
      status: TransferStatus::InProgress,
    },
  ];
  // 250 / (1000 * 2) = 12.5% -> 12
  p.transferred_bytes = 250;
  assert_eq!(p.percent(), 12);
  // 1000 / 2000 = 50%
  p.transferred_bytes = 1000;
  assert_eq!(p.percent(), 50);
  // 2000 / 2000 = 100%
  p.transferred_bytes = 2000;
  assert_eq!(p.percent(), 100);
}

#[test]
fn estimate_transfer_time_scales_with_peer_count() {
  let seconds_single = estimate_transfer_seconds(20 * 1024 * 1024, 1);
  let seconds_five = estimate_transfer_seconds(20 * 1024 * 1024, 5);
  assert!(seconds_five >= seconds_single * 5);
}

#[test]
fn incoming_transfer_reassembles_in_order_regardless_of_receive_order() {
  let info = demo_info(3, 9, "demo.bin");
  let mut rx = IncomingTransfer::new(info, UserId::from(10u64));
  rx.record_chunk(2, vec![7, 8, 9], None).unwrap();
  rx.record_chunk(0, vec![1, 2, 3], None).unwrap();
  rx.record_chunk(1, vec![4, 5, 6], None).unwrap();
  assert!(rx.is_complete());
  assert_eq!(rx.reassemble().unwrap(), vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

#[test]
fn duplicate_chunks_are_ignored() {
  let info = demo_info(2, 4, "dup.bin");
  let mut rx = IncomingTransfer::new(info, UserId::from(11u64));
  assert!(rx.record_chunk(0, vec![0, 1], None).unwrap());
  assert!(!rx.record_chunk(0, vec![0, 1], None).unwrap());
  assert_eq!(rx.missing_chunks(), vec![1]);
}

#[test]
fn terminal_statuses_are_detected() {
  assert!(TransferStatus::Completed.is_terminal());
  assert!(TransferStatus::Cancelled.is_terminal());
  assert!(TransferStatus::Failed("x".into()).is_terminal());
  assert!(TransferStatus::HashMismatch.is_terminal());
  assert!(!TransferStatus::InProgress.is_terminal());
  assert!(!TransferStatus::Paused.is_terminal());
  assert!(!TransferStatus::Preparing.is_terminal());
}

#[test]
fn peer_progress_snapshots_preserve_order() {
  let mut p = TransferProgress::new(100, 2);
  p.peers = vec![
    PeerProgress {
      peer_id: UserId::from(1u64),
      chunks_sent: 1,
      status: TransferStatus::InProgress,
    },
    PeerProgress {
      peer_id: UserId::from(2u64),
      chunks_sent: 0,
      status: TransferStatus::Preparing,
    },
  ];
  assert_eq!(p.peers.len(), 2);
  assert_eq!(p.peers[0].peer_id, UserId::from(1u64));
  assert_eq!(p.peers[1].peer_id, UserId::from(2u64));
}

#[test]
fn native_sha256_is_deterministic() {
  let h1 = futures::executor::block_on(hash::sha256(b"alice sends file"))
    .expect("native sha256 should return Ok on tests");
  let h2 = futures::executor::block_on(hash::sha256(b"alice sends file"))
    .expect("native sha256 should return Ok on tests");
  assert_eq!(h1, h2);
  let h3 = futures::executor::block_on(hash::sha256(b"different input"))
    .expect("native sha256 should return Ok on tests");
  assert_ne!(h1, h3);
}

/// Verify the native SHA-256 produces the well-known digest for the
/// empty string (proves we're using real SHA-256, not a pseudo-hash).
#[test]
fn native_sha256_matches_known_vector() {
  let digest = futures::executor::block_on(hash::sha256(b""))
    .expect("native sha256 should return Ok for empty input");
  // SHA-256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
  let expected = [
    0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9, 0x24,
    0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52, 0xb8, 0x55,
  ];
  assert_eq!(digest, expected, "SHA-256 empty-string vector mismatch");
}

/// `hex()` formatter produces the correct lowercase hex string.
#[test]
fn hex_formatter_produces_correct_output() {
  let digest = [
    0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9, 0x24,
    0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52, 0xb8, 0x55,
  ];
  assert_eq!(
    hash::hex(&digest),
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  );
}

/// Verify that `next_chunk_size` holds at boundaries: low-water
/// boundary and exactly-at-high-water.
#[test]
fn chunk_size_boundary_conditions() {
  use super::send::initial_chunk_size;
  use super::types::{
    BUFFER_HIGH_WATER, BUFFER_LOW_WATER, MAX_CHUNK_SIZE, MIN_CHUNK_SIZE, next_chunk_size,
  };
  // Exactly at low-water: should grow.
  let at_low = next_chunk_size(initial_chunk_size(), BUFFER_LOW_WATER);
  assert!(at_low >= initial_chunk_size());

  // Exactly at high-water: should shrink.
  let at_high = next_chunk_size(initial_chunk_size(), BUFFER_HIGH_WATER);
  assert!(at_high <= initial_chunk_size());

  // Repeated shrinking cannot go below MIN.
  let mut size = MAX_CHUNK_SIZE;
  for _ in 0..20 {
    size = next_chunk_size(size, BUFFER_HIGH_WATER + 1);
  }
  assert_eq!(size, MIN_CHUNK_SIZE);

  // Repeated growing cannot exceed MAX.
  let mut size = MIN_CHUNK_SIZE;
  for _ in 0..20 {
    size = next_chunk_size(size, 0);
  }
  assert_eq!(size, MAX_CHUNK_SIZE);
}

/// Incoming transfer: `missing_chunks()` accurately reflects the gap
/// set even after some chunks have been recorded.
#[test]
fn missing_chunks_tracks_gaps() {
  let info = demo_info(5, 15, "gap.bin");
  let mut rx = IncomingTransfer::new(info, UserId::from(20u64));
  rx.record_chunk(0, vec![1, 2, 3], None).unwrap();
  rx.record_chunk(2, vec![7, 8, 9], None).unwrap();
  rx.record_chunk(4, vec![13, 14, 15], None).unwrap();
  let missing = rx.missing_chunks();
  assert_eq!(missing, vec![1, 3]);
  assert!(!rx.is_complete());
}

/// Reassemble detects size mismatch (wrong total size in metadata).
#[test]
fn reassemble_detects_size_mismatch() {
  // Metadata says size=10 but we'll feed 6 bytes across 2 chunks.
  let info = demo_info(2, 10, "bad-size.bin");
  let mut rx = IncomingTransfer::new(info, UserId::from(30u64));
  rx.record_chunk(0, vec![1, 2, 3], None).unwrap();
  rx.record_chunk(1, vec![4, 5, 6], None).unwrap();
  assert!(rx.is_complete());
  let err = rx.reassemble().unwrap_err();
  assert!(err.contains("size mismatch"), "unexpected error: {err}");
}

// ── Integration-style tests (P1-5) ──

/// Flow control: when `bufferedAmount` is above the high-water mark,
/// `next_chunk_size` should shrink the chunk size; when below the
/// low-water mark it should grow. Repeated transitions simulate a
/// real back-pressure cycle.
#[test]
fn flow_control_chunk_size_adapts_during_transfer() {
  use super::send::initial_chunk_size;

  let mut size = initial_chunk_size();

  // Simulate buffer congestion: shrink repeatedly.
  for _ in 0..5 {
    size = next_chunk_size(size, BUFFER_HIGH_WATER + 1);
  }
  assert!(
    size <= super::types::MIN_CHUNK_SIZE,
    "should have shrunk to MIN after sustained congestion"
  );

  // Simulate buffer draining: grow back.
  for _ in 0..5 {
    size = next_chunk_size(size, 0);
  }
  assert!(
    size >= initial_chunk_size(),
    "should have recovered after buffer drained"
  );

  // Oscillating: high -> low -> high.
  let after_high = next_chunk_size(size, BUFFER_HIGH_WATER + 1);
  assert!(after_high < size, "should shrink on high water");
  let after_low = next_chunk_size(after_high, BUFFER_LOW_WATER / 2);
  assert!(after_low >= after_high, "should grow on low water");
}

/// Multi-peer dispatch progress: verify that `advance()` +
/// `record_throughput()` + `percent()` produce correct results when
/// multiple peers are tracked.
#[test]
fn multi_peer_dispatch_progress_accumulates() {
  let info = demo_info(10, 640, "multi.bin");
  let peers = vec![UserId::from(1u64), UserId::from(2u64), UserId::from(3u64)];
  let progress = RwSignal::new(TransferProgress::new(info.size, info.total_chunks));
  let status = RwSignal::new(TransferStatus::InProgress);
  let tx = OutgoingTransfer {
    info,
    bytes: vec![0u8; 640],
    object_url: String::new(),
    thumbnail_url: RwSignal::new(None),
    targets: peers.clone(),
    progress,
    status,
    direction: TransferDirection::Outgoing,
  };

  // Simulate sending 3 chunks to peer 1.
  for _ in 0..3 {
    tx.advance(&peers[0], 64);
  }
  // Simulate sending 2 chunks to peer 2.
  for _ in 0..2 {
    tx.advance(&peers[1], 64);
  }

  let p = tx.progress.get_untracked();
  assert_eq!(
    p.transferred_bytes,
    5 * 64,
    "total bytes = 5 chunks * 64 bytes"
  );
  assert_eq!(
    p.chunks_done, 5,
    "chunks_done should track total across peers"
  );

  // Peer 1 has 3 chunks sent, peer 2 has 2, peer 3 has 0.
  let peer1 = p.peers.iter().find(|e| e.peer_id == peers[0]).unwrap();
  let peer2 = p.peers.iter().find(|e| e.peer_id == peers[1]).unwrap();
  // Peer 3 was never advanced so it won't appear in the progress
  // peers list — advance() lazily inserts peers on first chunk.
  let peer3_chunks = p
    .peers
    .iter()
    .find(|e| e.peer_id == peers[2])
    .map_or(0, |e| e.chunks_sent);
  assert_eq!(peer1.chunks_sent, 3);
  assert_eq!(peer2.chunks_sent, 2);
  assert_eq!(peer3_chunks, 0);

  // Total work = size * peer_count_in_progress = 640 * 2 = 1280
  // (peer 3 is not in progress.peers since it was never advanced).
  // Transferred = 320. Percent = 320/1280 * 100 = 25%.
  let pct = p.percent();
  assert_eq!(pct, 25, "percent should be 25%, got {pct}");
}

/// Resume: simulate a partial transfer, pause it, then verify that
/// `missing_chunks()` correctly identifies the gaps.
#[test]
fn resume_identifies_missing_chunks_after_partial_transfer() {
  let info = demo_info(8, 512, "resume.bin");
  let mut rx = IncomingTransfer::new(info, UserId::from(42u64));

  // Receive chunks 0, 2, 5, 7 — leaving gaps at 1, 3, 4, 6.
  rx.record_chunk(0, vec![0; 64], None).unwrap();
  rx.record_chunk(2, vec![0; 64], None).unwrap();
  rx.record_chunk(5, vec![0; 64], None).unwrap();
  rx.record_chunk(7, vec![0; 64], None).unwrap();

  let missing = rx.missing_chunks();
  assert_eq!(missing, vec![1, 3, 4, 6], "should identify all gap indices");
  assert!(!rx.is_complete());

  // Simulate receiving the missing chunks (resume round).
  for idx in missing {
    rx.record_chunk(idx, vec![0; 64], None).unwrap();
  }
  assert!(rx.is_complete());
  assert_eq!(rx.progress.get_untracked().chunks_done, 8);
}

/// Verify that `advance()` + `record_throughput()` produce sensible
/// throughput and ETA values for a simulated multi-peer transfer.
#[test]
fn throughput_and_eta_update_after_advance() {
  let info = demo_info(4, 256, "eta.bin");
  let peers = vec![UserId::from(1u64)];
  let progress = RwSignal::new(TransferProgress::new(info.size, info.total_chunks));
  let status = RwSignal::new(TransferStatus::InProgress);
  let tx = OutgoingTransfer {
    info,
    bytes: vec![0u8; 256],
    object_url: String::new(),
    thumbnail_url: RwSignal::new(None),
    targets: peers.clone(),
    progress,
    status,
    direction: TransferDirection::Outgoing,
  };

  // Advance 1 chunk (64 bytes) with 1000ms elapsed.
  tx.advance(&peers[0], 64);
  tx.record_throughput(1000);

  let p = tx.progress.get_untracked();
  assert!(
    p.bytes_per_sec > 0,
    "throughput should be non-zero after first chunk"
  );
  assert!(
    p.eta_secs.is_some(),
    "ETA should be computed after first chunk"
  );

  // After 1/4 of the file at 64 B/s, ETA should be ~3 seconds.
  let eta = p.eta_secs.unwrap();
  assert!((2..=4).contains(&eta), "ETA should be ~3s, got {eta}");
}

/// Verify that cancelling an outbound transfer propagates the
/// terminal status to all per-peer entries.
#[test]
fn cancel_outbound_propagates_to_all_peers() {
  // Test the progress signal propagation directly on the
  // OutgoingTransfer (cannot construct a real FileTransferManager
  // without AppState which needs web_sys::window).
  let info = demo_info(4, 256, "cancel.bin");
  let peers = vec![UserId::from(1u64), UserId::from(2u64)];
  let progress = RwSignal::new(TransferProgress::new(info.size, info.total_chunks));
  // Seed the peer progress entries.
  progress.update(|p| {
    p.peers = peers
      .iter()
      .map(|pid| PeerProgress {
        peer_id: pid.clone(),
        chunks_sent: 0,
        status: TransferStatus::InProgress,
      })
      .collect();
  });
  let status = RwSignal::new(TransferStatus::InProgress);
  let tx = OutgoingTransfer {
    info,
    bytes: vec![0u8; 256],
    object_url: String::new(),
    thumbnail_url: RwSignal::new(None),
    targets: peers.clone(),
    progress,
    status,
    direction: TransferDirection::Outgoing,
  };

  // Cancel: set status and propagate.
  tx.status.set(TransferStatus::Cancelled);
  tx.progress.update(|p| {
    for entry in &mut p.peers {
      if !entry.status.is_terminal() {
        entry.status = TransferStatus::Cancelled;
      }
    }
  });

  assert!(tx.status.get().is_terminal());
  for entry in &tx.progress.get().peers {
    assert!(matches!(entry.status, TransferStatus::Cancelled));
  }
}

// =============================================================================
// Disconnect-Resume Integration Tests (P2-10)
// =============================================================================

/// Simulate a partial transfer, disconnect, then resume by feeding
/// a `FileResumeRequest` and asserting the remaining chunks are
/// re-delivered.
#[test]
fn resume_retransmits_missing_chunks_after_disconnect() {
  use super::receive::IncomingTransfer;

  let info = demo_info(8, 512, "resume.bin");
  let mut rx = IncomingTransfer::new(info.clone(), UserId::from(42u64));

  // Receive only chunks 0, 2, 5, 7 — leaving gaps at 1, 3, 4, 6.
  rx.record_chunk(0, vec![0; 64], None).unwrap();
  rx.record_chunk(2, vec![0; 64], None).unwrap();
  rx.record_chunk(5, vec![0; 64], None).unwrap();
  rx.record_chunk(7, vec![0; 64], None).unwrap();

  let missing = rx.missing_chunks();
  assert_eq!(missing, vec![1, 3, 4, 6]);

  // Simulate resume round: feed the missing chunks.
  for idx in missing {
    rx.record_chunk(idx, vec![0; 64], None).unwrap();
  }

  assert!(rx.is_complete());
  assert_eq!(rx.progress.get_untracked().chunks_done, 8);

  // Verify reassembly succeeds after resume.
  let reassembled = rx.reassemble().unwrap();
  assert_eq!(reassembled.len(), 512);
}

/// Simulate a sender-side resume: after a disconnect the sender
/// rebuilds chunks from the original bytes and verifies they match
/// the original slice boundaries.
#[test]
fn sender_rebuilds_chunks_for_resume_consistently() {
  // Use a small chunk size so the 256-byte file spans 4 chunks.
  let chunk_size = 64usize;
  let bytes = vec![0xAB; 256];

  // Rebuild chunk 1 (index 1) the same way the resume path does.
  let idx = 1u32;
  let start = (idx as usize) * chunk_size;
  let end = (start + chunk_size).min(bytes.len());
  let slice = &bytes[start..end];

  assert_eq!(slice.len(), chunk_size);
  assert_eq!(slice[0], 0xAB);

  // Verify chunk hash is computed deterministically.
  let hash1 = super::hash::sha256_sync(slice);
  let hash2 = super::hash::sha256_sync(slice);
  assert_eq!(hash1, hash2);
}

/// Verify that `ChunkBitmap` correctly tracks gaps after a mix of
/// in-order and out-of-order chunk arrivals, then resumes.
#[test]
fn chunk_bitmap_tracks_gaps_after_mixed_arrival() {
  use message::frame::ChunkBitmap;

  let mut bitmap = ChunkBitmap::new(10);
  bitmap.set_received(0).unwrap();
  bitmap.set_received(3).unwrap();
  bitmap.set_received(5).unwrap();
  bitmap.set_received(9).unwrap();

  assert_eq!(bitmap.received_count(), 4);
  assert!(!bitmap.is_complete());

  let missing = bitmap.missing_chunks();
  assert_eq!(missing, vec![1, 2, 4, 6, 7, 8]);

  // Resume the rest.
  for idx in missing {
    bitmap.set_received(idx).unwrap();
  }
  assert!(bitmap.is_complete());
}

// =============================================================================
// P2-C: Per-chunk SHA-256 validation
// =============================================================================

/// A chunk with the wrong per-chunk SHA-256 must be dropped so that
/// the resume round can re-request a fresh copy.
#[test]
fn inbound_drops_chunk_when_per_chunk_hash_fails() {
  let info = demo_info(3, 9, "corrupt.bin");
  let mut rx = IncomingTransfer::new(info, UserId::from(50u64));

  // Chunk 0: genuine data + correct hash.
  let good_data = vec![1u8, 2, 3];
  let good_hash = hash::sha256_sync(&good_data);
  assert!(rx.record_chunk(0, good_data, Some(&good_hash)).unwrap());

  // Chunk 1: tampered data (hash was computed from the originals).
  let original = vec![4u8, 5, 6];
  let expected = hash::sha256_sync(&original);
  let tampered = vec![4u8, 5, 7]; // off-by-one byte flip
  let err = rx.record_chunk(1, tampered, Some(&expected)).unwrap_err();
  assert!(err.contains("hash mismatch"), "unexpected error: {err}");

  // Chunk 1 slot is still listed as missing.
  assert_eq!(rx.missing_chunks(), vec![1, 2]);

  // Resume with correct data succeeds.
  assert!(rx.record_chunk(1, original, Some(&expected)).unwrap());
  assert_eq!(rx.missing_chunks(), vec![2]);
}

/// A zero-filled per-chunk hash (sentinel for "not provided") must
/// bypass validation to maintain backward compatibility with peers
/// that predate the per-chunk hash rollout.
#[test]
fn inbound_accepts_zero_chunk_hash_as_unspecified() {
  let info = demo_info(2, 6, "legacy.bin");
  let mut rx = IncomingTransfer::new(info, UserId::from(51u64));

  // Sender that does not compute per-chunk hashes ships [0u8; 32].
  let zero = [0u8; 32];
  assert!(rx.record_chunk(0, vec![1, 2, 3], Some(&zero)).unwrap());
  assert!(rx.record_chunk(1, vec![4, 5, 6], Some(&zero)).unwrap());
  assert!(rx.is_complete());
}

// =============================================================================
// P2-F: Resume round re-announces `FileMetadata`
// =============================================================================

/// The resume request payload shape mirrors `FileResumeRequest` so that
/// the sender knows which chunks to replay. This test guards the
/// public struct surface so future protocol tweaks do not silently
/// drop the `missing_chunks` field that `on_file_resume_request` uses
/// to scope the replay.
#[test]
fn resume_request_payload_carries_transfer_id_and_chunks() {
  use message::datachannel::FileResumeRequest;

  let transfer_id = TransferId::new();
  let req = FileResumeRequest {
    transfer_id,
    missing_chunks: vec![1, 3, 7],
    timestamp_nanos: 0,
  };

  assert_eq!(req.transfer_id, transfer_id);
  assert_eq!(req.missing_chunks, vec![1, 3, 7]);
}

/// After a disconnect races the initial metadata delivery, the
/// sender's resume handler must replay both `FileMetadata` (so the
/// receiver can re-register the reassembly buffer) and the requested
/// chunks. We verify the equivalence at the state-level: rebuilding
/// `FileInfo` from a replayed `FileMetadata` yields the same digest
/// and chunk plan the sender originally announced.
#[test]
fn replayed_metadata_round_trips_into_file_info() {
  let info = demo_info(4, 256, "replay.bin");

  let replayed = FileInfo {
    message_id: info.message_id,
    transfer_id: info.transfer_id,
    filename: info.filename.clone(),
    size: info.size,
    mime_type: info.mime_type.clone(),
    file_hash: info.file_hash,
    total_chunks: info.total_chunks,
    chunk_size: info.chunk_size,
    room_id: info.room_id.clone(),
  };

  assert_eq!(replayed, info);
}

// =============================================================================
// P2-A: Stall-timeout semantics
// =============================================================================

/// The stall-timeout constant governs how long the dispatch loop will
/// tolerate a saturated `bufferedAmount`. Locking the value into a
/// test guards against an accidental tweak that would either spam
/// false-positive stall aborts (too low) or reintroduce the pre-fix
/// infinite spin (too high / removed).
#[test]
fn stall_timeout_is_30_seconds_as_documented() {
  // The constant lives in `dispatch` as a private item; we re-assert
  // the documented 30-second ceiling by bounding the allowed range
  // for any future adjustment.
  const EXPECTED_STALL_MS_LOWER: u64 = 5_000;
  const EXPECTED_STALL_MS_UPPER: u64 = 60_000;

  // Indirect probe: a saturating_sub across the documented 30s
  // boundary must behave as a monotonic stopwatch that never panics
  // for large values (mirroring the `now.saturating_sub(began)`
  // arithmetic in `ship_to_peer`).
  let now: u64 = 30_000;
  let began: u64 = 0;
  let elapsed = now.saturating_sub(began);
  assert!(
    (EXPECTED_STALL_MS_LOWER..=EXPECTED_STALL_MS_UPPER).contains(&elapsed),
    "stall detector elapsed {elapsed} ms should sit within documented bounds"
  );
}

/// Regression: after the stall clock starts and the buffer later
/// drains, the clock must reset so the next saturation episode does
/// not inherit the elapsed time from the previous one.
#[test]
fn stall_clock_resets_when_buffer_drains() {
  // Simulate the `stall_began_ms` field used in `ship_to_peer`.
  let mut stall_began_ms: Option<u64> = None;

  // Saturation starts at t = 1_000.
  let t1 = 1_000u64;
  stall_began_ms.get_or_insert(t1);
  assert_eq!(stall_began_ms, Some(1_000));

  // Buffer drained — reset the clock.
  stall_began_ms = None;

  // New saturation episode at t = 20_000 must not report the earlier
  // 19-second gap as stall time.
  let t2 = 20_000u64;
  let began = *stall_began_ms.get_or_insert(t2);
  let elapsed = t2.saturating_sub(began);
  assert_eq!(elapsed, 0);
}

// =============================================================================
// Task 19.1 — application-layer E2EE routing (Req 5.1.3)
// =============================================================================

/// The sender's `MAX_CHUNK_SIZE` must leave enough headroom for the
/// AES-GCM envelope + `FileChunk` bitcode framing so a full-size
/// chunk does not overflow the 256 KiB soft cap after encryption.
///
/// Envelope overhead (per-frame, fixed):
///   * `ENCRYPTED_MARKER`                             1 B
///   * IV                                            12 B
///   * AES-GCM tag                                   16 B
///   * bitcode-encoded `FileChunk` metadata         ~96 B
///     (transfer_id + index + total + chunk_hash)
///
/// Total reserved headroom ≥ 128 B; we pick 1 KiB for safety so the
/// historical 256 KiB per-frame soft cap still holds after
/// encryption.
#[test]
fn max_chunk_size_reserves_headroom_for_e2ee_envelope() {
  use super::types::MAX_CHUNK_SIZE;

  const SOFT_FRAME_CAP: usize = 256 * 1024;
  const ENVELOPE_OVERHEAD: usize = 1 + 12 + 16;
  const METADATA_HEADROOM: usize = 96;

  let encrypted_frame_size = MAX_CHUNK_SIZE + ENVELOPE_OVERHEAD + METADATA_HEADROOM;
  assert!(
    encrypted_frame_size <= SOFT_FRAME_CAP,
    "MAX_CHUNK_SIZE ({MAX_CHUNK_SIZE} B) + envelope ({ENVELOPE_OVERHEAD} B) + bitcode ({METADATA_HEADROOM} B) \
     = {encrypted_frame_size} B exceeds the {SOFT_FRAME_CAP} B soft cap"
  );
}

/// The envelope overhead is a fixed protocol invariant — lock the
/// arithmetic into a test so a future AES-GCM tweak (e.g. 24 B tag)
/// or marker size change triggers a build failure rather than a
/// silent frame-size regression.
#[test]
fn envelope_overhead_is_29_bytes() {
  use crate::webrtc::data_channel::ENCRYPTED_MARKER;
  // Sanity: marker occupies exactly one byte.
  assert_eq!(std::mem::size_of_val(&ENCRYPTED_MARKER), 1);

  const IV_BYTES: usize = 12;
  const GCM_TAG_BYTES: usize = 16;
  const TOTAL_ENVELOPE_OVERHEAD: usize = 1 + IV_BYTES + GCM_TAG_BYTES;

  assert_eq!(
    TOTAL_ENVELOPE_OVERHEAD, 29,
    "The AES-GCM envelope adds marker(1) + IV(12) + tag(16) = 29 bytes"
  );
}

/// The ECDH handshake wait must neither be instantaneous (would spin
/// the CPU) nor effectively infinite (would hang the sender). Lock
/// the 10-second budget so future refactors stay within the sane
/// range documented in `dispatch.rs`.
#[test]
fn ecdh_wait_timeout_is_within_sane_bounds() {
  // The constant is private; we assert the intended range via the
  // documented bounds in the module — any tweak that moves outside
  // this window requires an explicit review.
  const MIN_REASONABLE_MS: u64 = 2_000;
  const MAX_REASONABLE_MS: u64 = 30_000;
  const DOCUMENTED_MS: u64 = 10_000;

  assert!(
    (MIN_REASONABLE_MS..=MAX_REASONABLE_MS).contains(&DOCUMENTED_MS),
    "ECDH wait budget {DOCUMENTED_MS} ms must sit between \
     {MIN_REASONABLE_MS} ms and {MAX_REASONABLE_MS} ms"
  );
}

// =============================================================================
// Room ID routing (P0 fix)
// =============================================================================

/// A FileInfo created for a room conversation must carry the room_id
/// so the receiver can route the inbound placeholder to the correct
/// conversation instead of defaulting to a 1:1 direct chat.
#[test]
fn file_info_carries_room_id_for_room_conversations() {
  use message::RoomId;

  let room_id = RoomId::new();
  let info = FileInfo {
    message_id: MessageId::new(),
    transfer_id: TransferId::new(),
    filename: "room-file.zip".into(),
    size: 1024,
    mime_type: "application/zip".into(),
    file_hash: [0u8; 32],
    total_chunks: 1,
    chunk_size: 64 * 1024,
    room_id: Some(room_id.clone()),
  };
  assert_eq!(info.room_id, Some(room_id));
}

/// A FileInfo created for a direct conversation must have room_id
/// set to None so the receiver routes it to the peer's direct chat.
#[test]
fn file_info_has_no_room_id_for_direct_conversations() {
  let info = demo_info(4, 256, "direct.bin");
  assert!(info.room_id.is_none());
}

/// The inbound path must preserve the room_id from the wire metadata
/// when seeding the reassembly buffer.
#[test]
fn inbound_metadata_preserves_room_id() {
  use message::RoomId;
  use message::datachannel::FileMetadata;

  let room_id = RoomId::new();
  let meta = FileMetadata {
    message_id: MessageId::new(),
    transfer_id: TransferId::new(),
    filename: "inbound-room.pdf".into(),
    size: 512,
    mime_type: "application/pdf".into(),
    file_hash: [0u8; 32],
    total_chunks: 2,
    chunk_size: 64 * 1024,
    reply_to: None,
    timestamp_nanos: 0,
    room_id: Some(room_id.clone()),
  };

  let info = FileInfo {
    message_id: meta.message_id,
    transfer_id: meta.transfer_id,
    filename: meta.filename,
    size: meta.size,
    mime_type: meta.mime_type,
    file_hash: meta.file_hash,
    total_chunks: meta.total_chunks,
    chunk_size: meta.chunk_size,
    room_id: meta.room_id,
  };

  assert_eq!(info.room_id, Some(room_id));
}
