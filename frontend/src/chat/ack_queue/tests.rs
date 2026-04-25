use super::*;

#[test]
fn track_and_acknowledge_single_peer() {
  let mut q = AckQueue::default();
  let id = MessageId::new();
  let peer = UserId::from(1u64);
  q.track(id, "conv".to_string(), vec![peer.clone()]);
  assert_eq!(q.len(), 1);
  assert!(q.acknowledge(&id, &peer));
  assert!(q.is_empty());
}

#[test]
fn acknowledge_partial_waits_for_remaining_peers() {
  let mut q = AckQueue::default();
  let id = MessageId::new();
  let a = UserId::from(1u64);
  let b = UserId::from(2u64);
  q.track(id, "room".to_string(), vec![a.clone(), b.clone()]);
  assert!(!q.acknowledge(&id, &a));
  assert_eq!(q.len(), 1);
  assert!(q.acknowledge(&id, &b));
  assert!(q.is_empty());
}

#[test]
fn retry_and_expire() {
  let mut q = AckQueue::default();
  let id = MessageId::new();
  q.track(id, "x".to_string(), vec![UserId::from(1u64)]);

  // Use a base time close to the entry's created_ms so we don't trigger
  // the 72-hour expiry check. Start with time slightly after next_retry_ms
  // and increment to force retries.
  let base_time = Utc::now().timestamp_millis();
  let mut results = Vec::new();
  let mut time = base_time + config::INITIAL_BACKOFF_MS + 1;
  for _ in 0..10 {
    let r = q.tick(time);
    results.extend(r);
    if q.is_empty() {
      break;
    }
    // Advance time by max backoff to ensure we hit the next retry window.
    time += config::MAX_BACKOFF_MS + 1;
  }
  assert!(results.iter().any(|(_, r)| *r == TickResult::Retry));
  assert!(results.iter().any(|(_, r)| *r == TickResult::Expired));
  assert!(q.is_empty());
}

#[test]
fn forget_removes_entry() {
  let mut q = AckQueue::default();
  let id = MessageId::new();
  q.track(id, "c".to_string(), vec![UserId::from(1u64)]);
  q.forget(&id);
  assert!(q.is_empty());
}

#[test]
fn entry_expires_after_72_hours() {
  let mut q = AckQueue::default();
  let id = MessageId::new();
  q.track(id, "conv".to_string(), vec![UserId::from(1u64)]);

  // Get entry and manually set a very old creation time.
  if let Some(entry) = q.entries.get_mut(&id) {
    entry.created_ms = 0; // Unix epoch — definitely more than 72h ago
  }

  // Now check that tick() marks it as expired.
  let now_ms = config::ACK_EXPIRY_MS + 1; // 72h + 1ms
  let results = q.tick(now_ms);
  assert_eq!(results.len(), 1);
  assert_eq!(results[0].1, TickResult::Expired);
  assert!(q.is_empty());
}

#[test]
fn cleanup_expired_removes_old_entries() {
  let mut q = AckQueue::default();
  let id1 = MessageId::new();
  let id2 = MessageId::new();
  q.track(id1, "conv1".to_string(), vec![UserId::from(1u64)]);
  q.track(id2, "conv2".to_string(), vec![UserId::from(2u64)]);

  // Set id1 to be very old.
  if let Some(entry) = q.entries.get_mut(&id1) {
    entry.created_ms = 0;
  }

  let now_ms = config::ACK_EXPIRY_MS + 1;
  let expired = q.cleanup_expired(now_ms);

  assert_eq!(expired.len(), 1);
  assert_eq!(expired[0], id1);
  assert_eq!(q.len(), 1);
  assert!(q.entries.contains_key(&id2));
}

#[test]
fn restore_entry_dedups_awaiting_peers() {
  let mut q = AckQueue::default();
  let id = MessageId::new();
  let peer = UserId::from(1u64);

  // First restore creates a new entry.
  let pending = Pending::new("conv".to_string(), vec![]);
  q.restore_entry(id, "conv".to_string(), peer.clone(), pending.clone(), 1_000);
  assert_eq!(q.len(), 1);
  {
    let entry = q.entries.get(&id).unwrap();
    assert_eq!(entry.awaiting.len(), 1);
    assert!(entry.awaiting.contains(&peer));
    assert_eq!(entry.conversation_key, "conv");
    assert_eq!(entry.created_ms, 1_000);
  }

  // Second restore with the same peer should dedup (not add duplicate).
  q.restore_entry(id, "conv".to_string(), peer.clone(), pending.clone(), 1_000);
  assert_eq!(q.len(), 1);
  {
    let entry = q.entries.get(&id).unwrap();
    assert_eq!(
      entry.awaiting.len(),
      1,
      "duplicate peer should be deduplicated"
    );
  }

  // Restore with a different peer appends to awaiting list.
  let peer2 = UserId::from(2u64);
  q.restore_entry(
    id,
    "conv".to_string(),
    peer2.clone(),
    pending.clone(),
    1_000,
  );
  assert_eq!(q.len(), 1);
  {
    let entry = q.entries.get(&id).unwrap();
    assert_eq!(entry.awaiting.len(), 2);
    assert!(entry.awaiting.contains(&peer));
    assert!(entry.awaiting.contains(&peer2));
  }
}

#[test]
fn restore_entry_creates_new_for_different_id() {
  let mut q = AckQueue::default();
  let id1 = MessageId::new();
  let id2 = MessageId::new();
  let peer = UserId::from(1u64);
  let pending = Pending::new("conv".to_string(), vec![]);

  q.restore_entry(
    id1,
    "conv".to_string(),
    peer.clone(),
    pending.clone(),
    1_000,
  );
  q.restore_entry(
    id2,
    "conv".to_string(),
    peer.clone(),
    pending.clone(),
    2_000,
  );

  assert_eq!(q.len(), 2);
  assert_eq!(q.entries.get(&id1).unwrap().created_ms, 1_000);
  assert_eq!(q.entries.get(&id2).unwrap().created_ms, 2_000);
}
