//! Retry queue for outbound messages awaiting a `MessageAck`.
//!
//! When the local user sends a chat message (text / sticker / voice /
//! image / forward) we keep it in this queue until:
//!
//! 1. A `MessageAck { status: Received }` arrives from a peer -> remove.
//! 2. The retry budget is exhausted -> mark as `Failed` in the UI.
//!
//! The queue is intentionally kept in memory only for Task 16. Persisting
//! to IndexedDB is a follow-up because the Web Crypto API requires the
//! session to stay alive for decryption anyway — restarting the page
//! re-runs the ECDH handshake and sends a fresh `MessageAck` round trip.

use chrono::Utc;
use message::{MessageId, UserId};
use std::collections::HashMap;

/// Retry configuration constants (Req 4.2.x).
pub mod config {
  /// Maximum number of retries before marking the message as `Failed`.
  pub const MAX_RETRIES: u8 = 3;
  /// Initial retry delay (exponential backoff base).
  pub const INITIAL_BACKOFF_MS: i64 = 2_000;
  /// Factor applied to the previous delay on each retry.
  pub const BACKOFF_FACTOR: i64 = 2;
  /// Hard cap on the backoff delay.
  pub const MAX_BACKOFF_MS: i64 = 30_000;
}

/// A single outstanding acknowledgement.
#[derive(Debug, Clone)]
pub struct Pending {
  /// Conversation the message belongs to.
  pub conversation_key: String,
  /// The peers that still owe us an ACK (mesh: all room members; 1:1: the peer).
  pub awaiting: Vec<UserId>,
  /// Number of retries already attempted.
  pub attempts: u8,
  /// Earliest timestamp (Unix ms) at which we should retry.
  pub next_retry_ms: i64,
}

impl Pending {
  fn new(conversation_key: String, awaiting: Vec<UserId>) -> Self {
    let first_retry = Utc::now().timestamp_millis() + config::INITIAL_BACKOFF_MS;
    Self {
      conversation_key,
      awaiting,
      attempts: 0,
      next_retry_ms: first_retry,
    }
  }

  /// Advance the exponential backoff after a failed retry.
  fn bump(&mut self) {
    self.attempts = self.attempts.saturating_add(1);
    let delay = (config::INITIAL_BACKOFF_MS
      .saturating_mul(config::BACKOFF_FACTOR.saturating_pow(self.attempts.into())))
    .min(config::MAX_BACKOFF_MS);
    self.next_retry_ms = Utc::now().timestamp_millis() + delay;
  }

  /// Whether the retry budget has been exhausted.
  #[must_use]
  pub const fn is_exhausted(&self) -> bool {
    self.attempts >= config::MAX_RETRIES
  }
}

/// Result of polling the queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TickResult {
  /// Nothing needs doing for this id at this time.
  Idle,
  /// The caller should resend the message.
  Retry,
  /// The retry budget is exhausted; mark the UI message as `Failed`.
  Expired,
}

/// In-memory ACK tracker.
#[derive(Debug, Clone, Default)]
pub struct AckQueue {
  entries: HashMap<MessageId, Pending>,
}

impl AckQueue {
  /// Register a new outbound message.
  pub fn track(&mut self, id: MessageId, conversation_key: String, peers: Vec<UserId>) {
    if peers.is_empty() {
      return;
    }
    self
      .entries
      .insert(id, Pending::new(conversation_key, peers));
  }

  /// Record a successful ACK from `peer`. Returns `true` if the entry is
  /// now fully acknowledged by every expected peer (and was therefore
  /// removed).
  pub fn acknowledge(&mut self, id: &MessageId, peer: &UserId) -> bool {
    let mut drained = false;
    if let Some(entry) = self.entries.get_mut(id) {
      entry.awaiting.retain(|p| p != peer);
      if entry.awaiting.is_empty() {
        drained = true;
      }
    }
    if drained {
      self.entries.remove(id);
    }
    drained
  }

  /// Drop a pending entry (e.g. after the user revoked the message).
  pub fn forget(&mut self, id: &MessageId) {
    self.entries.remove(id);
  }

  /// Poll the queue at the current time. Returns entries whose
  /// `next_retry_ms` has been reached along with a classification.
  pub fn tick(&mut self, now_ms: i64) -> Vec<(MessageId, TickResult)> {
    let mut out = Vec::new();
    let mut expired = Vec::new();
    for (id, entry) in self.entries.iter_mut() {
      if now_ms < entry.next_retry_ms {
        continue;
      }
      if entry.is_exhausted() {
        out.push((*id, TickResult::Expired));
        expired.push(*id);
      } else {
        entry.bump();
        out.push((*id, TickResult::Retry));
      }
    }
    for id in expired {
      self.entries.remove(&id);
    }
    out
  }

  /// Number of pending entries (used by tests + debug panel).
  #[must_use]
  pub fn len(&self) -> usize {
    self.entries.len()
  }

  /// Whether the queue is empty.
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.entries.is_empty()
  }

  /// Iterate over tracked ids (debug helper).
  pub fn ids(&self) -> impl Iterator<Item = &MessageId> {
    self.entries.keys()
  }
}

// ── Tests ──

#[cfg(test)]
mod tests {
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

    // Large now => immediate retries until exhausted.
    let mut results = Vec::new();
    for _ in 0..10 {
      let r = q.tick(i64::MAX / 2);
      results.extend(r);
      if q.is_empty() {
        break;
      }
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
}
