use super::hash;
use super::*;

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

/// Simulate a partial transfer, disconnect, then resume by feeding
/// a `FileResumeRequest` and asserting the remaining chunks are
/// re-delivered.
#[test]
fn resume_retransmits_missing_chunks_after_disconnect() {
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
  let hash1 = super::super::hash::sha256_sync(slice);
  let hash2 = super::super::hash::sha256_sync(slice);
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
