//! End-to-end danmaku relay pipeline tests (Req 12.5 §28).
//!
//! These tests simulate the full owner-side relay flow:
//!
//! 1. Viewer danmaku arrive (via `dc_router::apply` or direct enqueue).
//! 2. The owner's `DanmakuBatcher` collects them.
//! 3. A periodic drain (simulating the 50 ms `set_interval`) flushes
//!    batches for broadcast.
//! 4. The output preserves FIFO order, respects `MAX_BATCH_SIZE`, and
//!    correctly enforces overload rate limiting.
//!
//! These are native tests — no browser required — because both the
//! batcher and the dc_router classify/dispatch logic are pure Rust.

use message::datachannel::Danmaku;
use message::types::DanmakuPosition;

use super::{DanmakuBatcher, MAX_BATCH_SIZE, OVERLOAD_LIMIT_PER_SECOND, RENDER_QUEUE_CAPACITY};

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

/// Simulate the full relay pipeline: N viewers each send M danmaku,
/// the owner batches and drains them. Verify all danmaku are relayed
/// in FIFO order.
#[test]
fn relay_pipeline_preserves_fifo_across_multiple_viewers() {
  let mut batcher = DanmakuBatcher::new();
  let viewer_count = 5;
  let msgs_per_viewer = 10;

  // Simulate interleaved arrivals from multiple viewers.
  for round in 0..msgs_per_viewer {
    for viewer in 0..viewer_count {
      let content = format!("v{viewer}_m{round}");
      assert!(batcher.enqueue(make_danmaku(&content)));
    }
  }

  let total = viewer_count * msgs_per_viewer;
  assert_eq!(batcher.pending_len(), total);

  // Drain all in batches (simulating multiple 50ms ticks).
  let mut all_output: Vec<Danmaku> = Vec::new();
  while batcher.pending_len() > 0 {
    let batch = batcher.drain_batch();
    assert!(!batch.is_empty());
    assert!(batch.len() <= MAX_BATCH_SIZE);
    all_output.extend(batch);
  }

  assert_eq!(all_output.len(), total);

  // Verify FIFO order: round 0 from all viewers comes before round 1.
  let mut idx = 0;
  for round in 0..msgs_per_viewer {
    for viewer in 0..viewer_count {
      let expected = format!("v{viewer}_m{round}");
      assert_eq!(all_output[idx].content, expected, "mismatch at index {idx}");
      idx += 1;
    }
  }
}

/// Simulate a burst that exceeds MAX_BATCH_SIZE: verify the relay
/// correctly splits across multiple drain cycles.
#[test]
fn relay_pipeline_splits_large_bursts_across_drains() {
  let mut batcher = DanmakuBatcher::new();
  let burst_size = MAX_BATCH_SIZE * 3 + 7; // e.g. 199

  for i in 0..burst_size {
    batcher.enqueue(make_danmaku(&format!("burst_{i}")));
  }
  assert_eq!(batcher.pending_len(), burst_size);

  // First drain: exactly MAX_BATCH_SIZE.
  let batch1 = batcher.drain_batch();
  assert_eq!(batch1.len(), MAX_BATCH_SIZE);
  assert_eq!(batch1[0].content, "burst_0");

  // Second drain: another MAX_BATCH_SIZE.
  let batch2 = batcher.drain_batch();
  assert_eq!(batch2.len(), MAX_BATCH_SIZE);
  assert_eq!(batch2[0].content, format!("burst_{MAX_BATCH_SIZE}"));

  // Third drain: another MAX_BATCH_SIZE.
  let batch3 = batcher.drain_batch();
  assert_eq!(batch3.len(), MAX_BATCH_SIZE);

  // Fourth drain: the remaining 7.
  let batch4 = batcher.drain_batch();
  assert_eq!(batch4.len(), 7);
  assert_eq!(batcher.pending_len(), 0);

  // Verify continuity.
  let total: usize = batch1.len() + batch2.len() + batch3.len() + batch4.len();
  assert_eq!(total, burst_size);
}

/// Simulate the overload scenario: when the owner is under high load,
/// the relay drops excess danmaku beyond the rate limit while still
/// delivering those within the window.
#[test]
fn relay_pipeline_overload_drops_excess_preserves_accepted() {
  let mut batcher = DanmakuBatcher::new();
  batcher.set_overload_mode(true);

  let now_ns: i64 = 1_000_000_000; // 1 second mark

  // Enqueue exactly the limit — all should be accepted.
  for i in 0..OVERLOAD_LIMIT_PER_SECOND {
    assert!(
      batcher.enqueue_with_now(make_danmaku(&format!("ok_{i}")), now_ns),
      "entry {i} should be accepted"
    );
  }

  // Drain to populate the dispatch window.
  let batch = batcher.drain_batch_with_now(now_ns);
  assert_eq!(batch.len(), OVERLOAD_LIMIT_PER_SECOND);

  // Now try to enqueue more within the same second — should be dropped.
  let excess_count = 10;
  for i in 0..excess_count {
    let accepted = batcher.enqueue_with_now(make_danmaku(&format!("drop_{i}")), now_ns);
    assert!(!accepted, "excess entry {i} should be dropped");
  }
  assert_eq!(batcher.dropped_count(), excess_count as u64);
  assert_eq!(batcher.pending_len(), 0);

  // After the 1-second window rolls over, new entries should be accepted.
  let next_second = now_ns + 1_000_000_001;
  for i in 0..5 {
    assert!(
      batcher.enqueue_with_now(make_danmaku(&format!("next_{i}")), next_second),
      "entry after window roll should be accepted"
    );
  }
  assert_eq!(batcher.pending_len(), 5);

  let recovered_batch = batcher.drain_batch_with_now(next_second);
  assert_eq!(recovered_batch.len(), 5);
  assert_eq!(recovered_batch[0].content, "next_0");
}

/// Simulate the relay pipeline with capacity eviction: when the queue
/// is full, oldest entries are evicted but the relay still delivers
/// the most recent entries correctly.
#[test]
fn relay_pipeline_eviction_under_capacity_pressure() {
  let mut batcher = DanmakuBatcher::new();

  // Fill to capacity + overflow.
  let overflow = 20;
  let total_enqueued = RENDER_QUEUE_CAPACITY + overflow;
  for i in 0..total_enqueued {
    batcher.enqueue(make_danmaku(&format!("msg_{i}")));
  }

  // Queue should be at capacity (oldest evicted).
  assert_eq!(batcher.pending_len(), RENDER_QUEUE_CAPACITY);

  // Drain everything.
  let mut all: Vec<Danmaku> = Vec::new();
  while batcher.pending_len() > 0 {
    all.extend(batcher.drain_batch());
  }

  assert_eq!(all.len(), RENDER_QUEUE_CAPACITY);
  // The first `overflow` entries should have been evicted.
  assert_eq!(all[0].content, format!("msg_{overflow}"));
  assert_eq!(
    all.last().unwrap().content,
    format!("msg_{}", total_enqueued - 1)
  );
}

/// Simulate rapid enqueue-drain cycles (mimicking the 50ms timer)
/// with interleaved arrivals between drains.
#[test]
fn relay_pipeline_interleaved_enqueue_drain_cycles() {
  let mut batcher = DanmakuBatcher::new();
  let mut all_output: Vec<Danmaku> = Vec::new();
  let cycles = 10;
  let per_cycle = 8;

  for cycle in 0..cycles {
    // Simulate arrivals between drain ticks.
    for i in 0..per_cycle {
      batcher.enqueue(make_danmaku(&format!("c{cycle}_d{i}")));
    }
    // Drain (simulating the 50ms timer firing).
    let batch = batcher.drain_batch();
    all_output.extend(batch);
  }

  // All entries should have been drained.
  assert_eq!(batcher.pending_len(), 0);
  assert_eq!(all_output.len(), cycles * per_cycle);

  // Verify order.
  let mut idx = 0;
  for cycle in 0..cycles {
    for i in 0..per_cycle {
      assert_eq!(all_output[idx].content, format!("c{cycle}_d{i}"));
      idx += 1;
    }
  }
}

/// Verify that drain on an empty batcher returns an empty vec and
/// does not panic.
#[test]
fn relay_pipeline_drain_empty_is_noop() {
  let mut batcher = DanmakuBatcher::new();
  let batch = batcher.drain_batch();
  assert!(batch.is_empty());
  assert_eq!(batcher.pending_len(), 0);
  assert_eq!(batcher.dropped_count(), 0);
}
