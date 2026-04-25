//! Batched read-receipt dispatcher.
//!
//! The spec (Req 4.3.x) requires grouping read receipts in a 500 ms
//! window so users that scroll through history do not flood peers with
//! individual `MessageRead` messages. This module buffers message ids per
//! peer and exposes a `drain_ready` call that returns the batches whose
//! window has elapsed.

use chrono::Utc;
use message::{MessageId, UserId};
use std::collections::HashMap;

/// Batching window in milliseconds (Req 4.3.x).
pub const BATCH_WINDOW_MS: i64 = 500;

/// Pending read-receipt batch for a single peer.
#[derive(Debug, Clone)]
struct Batch {
  message_ids: Vec<MessageId>,
  /// Unix ms at which this batch was first populated.
  first_at: i64,
}

/// Per-peer batch accumulator.
#[derive(Debug, Clone, Default)]
pub struct ReadBatcher {
  pending: HashMap<UserId, Batch>,
}

impl ReadBatcher {
  /// Record a message id as read against `peer`.
  pub fn mark_read(&mut self, peer: UserId, id: MessageId) {
    let now = Utc::now().timestamp_millis();
    let batch = self.pending.entry(peer).or_insert_with(|| Batch {
      message_ids: Vec::new(),
      first_at: now,
    });
    if !batch.message_ids.contains(&id) {
      batch.message_ids.push(id);
    }
  }

  /// Drain batches whose 500 ms window has elapsed. Returns `(peer, ids)`
  /// tuples ready for a `MessageRead` DataChannel message.
  pub fn drain_ready(&mut self, now_ms: i64) -> Vec<(UserId, Vec<MessageId>)> {
    let mut ready = Vec::new();
    let ready_keys: Vec<UserId> = self
      .pending
      .iter()
      .filter(|(_, b)| now_ms.saturating_sub(b.first_at) >= BATCH_WINDOW_MS)
      .map(|(k, _)| k.clone())
      .collect();
    for key in ready_keys {
      if let Some(batch) = self.pending.remove(&key) {
        ready.push((key, batch.message_ids));
      }
    }
    ready
  }

  /// Force-drain all pending batches (used when the conversation closes
  /// so no read receipt is lost). Returns `(peer, ids)` pairs in no
  /// particular order.
  pub fn drain_all(&mut self) -> Vec<(UserId, Vec<MessageId>)> {
    self
      .pending
      .drain()
      .map(|(k, b)| (k, b.message_ids))
      .collect()
  }

  /// Number of peers with pending batches.
  #[must_use]
  pub fn len(&self) -> usize {
    self.pending.len()
  }

  /// Whether there is nothing queued.
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.pending.is_empty()
  }
}

#[cfg(test)]
mod tests;
