//! File transfer protocol tests

use message::transfer;

#[test]
fn test_transfer_meta_serialize_roundtrip() {
  let meta = transfer::TransferMeta::new(
    "video.mp4".to_string(),
    50_000_000,
    "video/mp4".to_string(),
    transfer::DEFAULT_CHUNK_SIZE,
  );
  let bytes = bitcode::serialize(&meta).expect("serialization failed");
  let decoded: transfer::TransferMeta =
    bitcode::deserialize(&bytes).expect("deserialization failed");
  assert_eq!(decoded.file_name, "video.mp4");
  assert_eq!(decoded.file_size, 50_000_000);
  assert_eq!(decoded.chunk_size, transfer::DEFAULT_CHUNK_SIZE);
  assert!(decoded.total_chunks > 0);
}

#[test]
fn test_file_control_serialize_roundtrip() {
  let controls = vec![
    transfer::FileControl::Accept {
      transfer_id: "t-1".to_string(),
    },
    transfer::FileControl::Reject {
      transfer_id: "t-2".to_string(),
      reason: Some("file too large".to_string()),
    },
    transfer::FileControl::Complete {
      transfer_id: "t-3".to_string(),
    },
    transfer::FileControl::Cancel {
      transfer_id: "t-4".to_string(),
      reason: None,
    },
    transfer::FileControl::AdjustChunkSize {
      transfer_id: "t-5".to_string(),
      new_chunk_size: 128 * 1024,
    },
  ];
  for ctrl in &controls {
    let bytes = bitcode::serialize(ctrl).expect("serialization failed");
    let decoded: transfer::FileControl =
      bitcode::deserialize(&bytes).expect("deserialization failed");
    assert_eq!(&decoded, ctrl);
  }
}
