//! Danmaku (bullet-comment) dispatcher for Theater mode (Req 12.5).
//!
//! The danmaku data path is structured around **three transport roles**:
//!
//! * **Viewer → Owner** — each viewer sends its own danmaku to the
//!   owner through the single DataChannel established by the star
//!   topology. Viewers do not attempt to fan out themselves.
//! * **Owner relay** — the owner batches inbound viewer danmaku and its
//!   own danmaku into groups that fire every 50 ms, then forwards each
//!   batch to every other viewer. Batching keeps the per-peer send
//!   count low without perceptibly increasing end-to-end latency
//!   (< 500 ms target, Req 12.5 §28).
//! * **Viewer render** — every client (owner and viewers) renders
//!   incoming danmaku on the overlay canvas regardless of which peer
//!   produced them.
//!
//! This module is intentionally pure Rust so it can be unit-tested
//! without a browser — the UI layer handles canvas rendering and the
//! WebRTC layer handles DataChannel dispatch.

use std::collections::VecDeque;

use chrono::Utc;
use message::datachannel::Danmaku;

/// Maximum number of danmaku stored in the live render queue.
/// Additional entries evict the oldest (FIFO) to stay below the
/// Req 12.5 §24 "50 simultaneously displayed" guideline once render
/// lifetime is factored in.
pub const RENDER_QUEUE_CAPACITY: usize = 200;

/// Hard cap on batch size per flush. When the owner's inbound load is
/// extremely high we still cap per-batch forwarding to avoid overflowing
/// the DataChannel in a single send (Req 12.5 §28 overload fallback).
pub const MAX_BATCH_SIZE: usize = 64;

/// Overload threshold — max danmaku the owner forwards per second
/// before the "high load" fallback kicks in (Req 12.5 §28). Extra
/// danmaku are dropped FIFO by [`DanmakuBatcher::enqueue`] when
/// `overload_mode` is `true`.
pub const OVERLOAD_LIMIT_PER_SECOND: usize = 20;

/// A single danmaku entry with an ingestion timestamp used to enforce
/// the owner-relay rate limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DanmakuEntry {
  /// Wall-clock timestamp (nanoseconds) when this entry was enqueued.
  pub ingested_at_ns: i64,
  /// Underlying danmaku payload (ready for DataChannel transport).
  pub danmaku: Danmaku,
}

/// Pending-danmaku queue with 50 ms batch-merge semantics.
///
/// The batcher is reused by the owner on **both** edges of the
/// relay pipeline:
///
/// * Inbound — danmaku arriving from viewers are passed through
///   [`Self::enqueue`] and periodically drained via
///   [`Self::drain_batch`] to be forwarded.
/// * Outbound — the owner's own danmaku are enqueued the same way,
///   so the fan-out loop does not need to treat "self" as a special
///   case.
#[derive(Debug, Default)]
pub struct DanmakuBatcher {
  queue: VecDeque<DanmakuEntry>,
  /// Number of danmaku dispatched in the last rolling one-second
  /// window (used to enforce [`OVERLOAD_LIMIT_PER_SECOND`]).
  recent_dispatch_ts_ns: VecDeque<i64>,
  /// Whether the caller has declared the owner is currently
  /// overloaded (see [`Self::set_overload_mode`]).
  overload_mode: bool,
  /// Total number of danmaku that were dropped because the rate limit
  /// kicked in. Exposed for telemetry / test assertions.
  dropped: u64,
}

impl DanmakuBatcher {
  /// Create an empty batcher.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// Current queue depth (not yet flushed).
  #[must_use]
  pub fn pending_len(&self) -> usize {
    self.queue.len()
  }

  /// Number of danmaku that were dropped so far because the overload
  /// rate limit ([`OVERLOAD_LIMIT_PER_SECOND`]) was active.
  #[must_use]
  pub fn dropped_count(&self) -> u64 {
    self.dropped
  }

  /// Whether overload mode is currently active.
  #[must_use]
  pub fn is_overloaded(&self) -> bool {
    self.overload_mode
  }

  /// Toggle overload mode — when `true`, [`Self::enqueue`] enforces
  /// the [`OVERLOAD_LIMIT_PER_SECOND`] cap (FIFO drop).
  pub fn set_overload_mode(&mut self, enabled: bool) {
    self.overload_mode = enabled;
  }

  /// Push a new danmaku into the pending queue.
  ///
  /// Returns `true` when the entry was accepted, `false` when it was
  /// dropped because the overload limiter kicked in or the render
  /// queue reached [`RENDER_QUEUE_CAPACITY`].
  pub fn enqueue(&mut self, danmaku: Danmaku) -> bool {
    self.enqueue_with_now(danmaku, now_ns())
  }

  /// Same as [`Self::enqueue`] but with an explicit "now" timestamp
  /// (used by unit tests to make the rate limiter deterministic).
  pub fn enqueue_with_now(&mut self, danmaku: Danmaku, now_ns: i64) -> bool {
    if self.overload_mode {
      self.purge_old_dispatch(now_ns);
      if self.recent_dispatch_ts_ns.len() >= OVERLOAD_LIMIT_PER_SECOND {
        self.dropped = self.dropped.saturating_add(1);
        return false;
      }
    }
    if self.queue.len() >= RENDER_QUEUE_CAPACITY {
      self.queue.pop_front();
    }
    self.queue.push_back(DanmakuEntry {
      ingested_at_ns: now_ns,
      danmaku,
    });
    true
  }

  /// Drain the current queue into a single batch, respecting the
  /// [`MAX_BATCH_SIZE`] per-flush cap. Returns an empty vector when no
  /// danmaku are pending.
  pub fn drain_batch(&mut self) -> Vec<Danmaku> {
    self.drain_batch_with_now(now_ns())
  }

  /// Same as [`Self::drain_batch`] with an explicit "now" timestamp.
  pub fn drain_batch_with_now(&mut self, now_ns: i64) -> Vec<Danmaku> {
    let take = self.queue.len().min(MAX_BATCH_SIZE);
    let mut batch = Vec::with_capacity(take);
    for _ in 0..take {
      if let Some(entry) = self.queue.pop_front() {
        self.recent_dispatch_ts_ns.push_back(now_ns);
        batch.push(entry.danmaku);
      }
    }
    self.purge_old_dispatch(now_ns);
    batch
  }

  /// Remove all pending entries without dispatching.
  pub fn clear(&mut self) {
    self.queue.clear();
  }

  fn purge_old_dispatch(&mut self, now_ns: i64) {
    // One second window for the rate limiter.
    let cutoff = now_ns - 1_000_000_000;
    while self
      .recent_dispatch_ts_ns
      .front()
      .is_some_and(|ts| *ts < cutoff)
    {
      self.recent_dispatch_ts_ns.pop_front();
    }
  }
}

/// Current Unix timestamp in nanoseconds.
fn now_ns() -> i64 {
  Utc::now()
    .timestamp_nanos_opt()
    .unwrap_or_else(|| Utc::now().timestamp_micros() * 1_000)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod relay_pipeline_tests;
