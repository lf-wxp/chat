use super::*;

#[test]
fn marks_and_drains_after_window() {
  let mut b = ReadBatcher::default();
  let peer = UserId::from(1u64);
  let id = MessageId::new();
  b.mark_read(peer.clone(), id);
  // Nothing ready yet (now_ms close to first_at).
  assert!(b.drain_ready(Utc::now().timestamp_millis()).is_empty());
  // 1 second later -> ready.
  let later = Utc::now().timestamp_millis() + BATCH_WINDOW_MS + 1;
  let drained = b.drain_ready(later);
  assert_eq!(drained.len(), 1);
  assert_eq!(drained[0].1.len(), 1);
  assert_eq!(drained[0].1[0], id);
  assert!(b.is_empty());
}

#[test]
fn deduplicates_ids_per_peer() {
  let mut b = ReadBatcher::default();
  let peer = UserId::from(1u64);
  let id = MessageId::new();
  b.mark_read(peer.clone(), id);
  b.mark_read(peer.clone(), id);
  let all = b.drain_all();
  assert_eq!(all[0].1.len(), 1);
}

#[test]
fn drain_all_empties_queue() {
  let mut b = ReadBatcher::default();
  b.mark_read(UserId::from(1u64), MessageId::new());
  b.mark_read(UserId::from(2u64), MessageId::new());
  let drained = b.drain_all();
  assert_eq!(drained.len(), 2);
  assert!(b.is_empty());
}
