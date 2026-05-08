use super::*;

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
