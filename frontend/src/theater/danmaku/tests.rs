//! Unit tests for the Theater danmaku batcher.

use super::*;
use message::types::DanmakuPosition;

fn make_danmaku(content: &str) -> Danmaku {
  Danmaku {
    content: content.to_string(),
    font_size: 20,
    color: 0x00FF_FFFF,
    position: DanmakuPosition::Scroll,
    video_time_ms: 0,
    timestamp_nanos: 0,
  }
}

#[test]
fn enqueue_and_drain_preserves_order() {
  let mut batcher = DanmakuBatcher::new();
  for i in 0..5 {
    assert!(batcher.enqueue(make_danmaku(&format!("d{i}"))));
  }
  assert_eq!(batcher.pending_len(), 5);
  let batch = batcher.drain_batch();
  assert_eq!(batch.len(), 5);
  for (idx, d) in batch.iter().enumerate() {
    assert_eq!(d.content, format!("d{idx}"));
  }
  assert_eq!(batcher.pending_len(), 0);
}

#[test]
fn drain_respects_max_batch_size() {
  let mut batcher = DanmakuBatcher::new();
  for i in 0..(MAX_BATCH_SIZE + 10) {
    batcher.enqueue(make_danmaku(&format!("d{i}")));
  }
  let batch = batcher.drain_batch();
  assert_eq!(batch.len(), MAX_BATCH_SIZE);
  assert_eq!(batcher.pending_len(), 10);
}

#[test]
fn render_queue_evicts_oldest_when_capacity_reached() {
  let mut batcher = DanmakuBatcher::new();
  for i in 0..(RENDER_QUEUE_CAPACITY + 5) {
    batcher.enqueue(make_danmaku(&format!("d{i}")));
  }
  assert_eq!(batcher.pending_len(), RENDER_QUEUE_CAPACITY);
  let first = batcher.queue.front().unwrap();
  assert_eq!(first.danmaku.content, "d5", "oldest 5 should be evicted");
}

#[test]
fn overload_mode_drops_when_rate_exceeds_limit() {
  let mut batcher = DanmakuBatcher::new();
  batcher.set_overload_mode(true);
  let start = 0_i64;

  // First drain OVERLOAD_LIMIT_PER_SECOND entries and flush them to
  // populate the dispatch window.
  for i in 0..OVERLOAD_LIMIT_PER_SECOND {
    assert!(batcher.enqueue_with_now(make_danmaku(&format!("d{i}")), start));
  }
  let batch = batcher.drain_batch_with_now(start);
  assert_eq!(batch.len(), OVERLOAD_LIMIT_PER_SECOND);

  // Now enqueue at the same virtual "now" — the dispatch window is
  // full so new entries must be rejected.
  let accepted = batcher.enqueue_with_now(make_danmaku("extra"), start);
  assert!(!accepted);
  assert_eq!(batcher.dropped_count(), 1);

  // After the 1-second window rolls over, new entries should be
  // accepted again.
  let later = start + 1_000_000_001;
  assert!(batcher.enqueue_with_now(make_danmaku("next"), later));
}

#[test]
fn clear_empties_the_queue() {
  let mut batcher = DanmakuBatcher::new();
  for i in 0..3 {
    batcher.enqueue(make_danmaku(&format!("d{i}")));
  }
  batcher.clear();
  assert_eq!(batcher.pending_len(), 0);
}

#[test]
fn non_overload_mode_accepts_unlimited_entries() {
  let mut batcher = DanmakuBatcher::new();
  assert!(!batcher.is_overloaded());
  for i in 0..100 {
    assert!(batcher.enqueue(make_danmaku(&format!("d{i}"))));
  }
  assert_eq!(batcher.dropped_count(), 0);
}
