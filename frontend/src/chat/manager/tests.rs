//! Unit tests for `chat::manager` — pure-logic functions only.
//!
//! These tests exercise free functions and `Inner` methods that do not
//! depend on a live browser environment (WebSocket / WebRTC / DOM).

use super::*;
use crate::chat::models::{
  ChatMessage, FileRef, ImageRef, MessageContent, MessageStatus, StickerRef, VoiceClip,
};
use message::datachannel::DataChannelMessage;
use message::{MessageId, UserId};
use std::collections::BTreeMap;

// ── now_ms_to_nanos ──────────────────────────────────────────────

#[test]
fn now_ms_to_nanos_positive() {
  assert_eq!(now_ms_to_nanos(1), 1_000_000);
  assert_eq!(now_ms_to_nanos(1_000), 1_000_000_000);
}

#[test]
fn now_ms_to_nanos_zero() {
  assert_eq!(now_ms_to_nanos(0), 0);
}

#[test]
fn now_ms_to_nanos_negative_clamps_to_zero() {
  // Negative timestamps (before Unix epoch) are clamped to 0.
  assert_eq!(now_ms_to_nanos(-1), 0);
  assert_eq!(now_ms_to_nanos(-1_000_000), 0);
}

#[test]
fn now_ms_to_nanos_large_value() {
  // A typical "year 2030" Unix-ms value should not overflow.
  let y2030_ms: i64 = 1_895_000_000_000;
  let nanos = now_ms_to_nanos(y2030_ms);
  assert_eq!(nanos, (y2030_ms as u64) * 1_000_000);
}

// ── preview_for ──────────────────────────────────────────────────

fn make_msg(content: MessageContent) -> ChatMessage {
  ChatMessage {
    id: MessageId::new(),
    sender: UserId::from(1u64),
    sender_name: "Alice".to_string(),
    content,
    timestamp_ms: 0,
    outgoing: true,
    status: MessageStatus::Sent,
    reply_to: None,
    read_by: Vec::new(),
    reactions: BTreeMap::new(),
    mentions_me: false,
    counted_unread: false,
  }
}

#[test]
fn preview_for_text() {
  let msg = make_msg(MessageContent::Text("hello world".to_string()));
  assert_eq!(preview_for(&msg), "hello world");
}

#[test]
fn preview_for_sticker() {
  let msg = make_msg(MessageContent::Sticker(StickerRef {
    pack_id: "cats".to_string(),
    sticker_id: "smile".to_string(),
  }));
  assert_eq!(preview_for(&msg), "[Sticker]");
}

#[test]
fn preview_for_voice() {
  let msg = make_msg(MessageContent::Voice(VoiceClip {
    object_url: "blob:test".to_string(),
    duration_ms: 5000,
    waveform: vec![128; 50],
  }));
  assert_eq!(preview_for(&msg), "[Voice]");
}

#[test]
fn preview_for_image() {
  let msg = make_msg(MessageContent::Image(ImageRef {
    object_url: "blob:test".to_string(),
    thumbnail_url: "blob:thumb".to_string(),
    width: 800,
    height: 600,
  }));
  assert_eq!(preview_for(&msg), "[Image]");
}

#[test]
fn preview_for_file() {
  let msg = make_msg(MessageContent::File(FileRef {
    filename: "report.pdf".to_string(),
    size: 1024,
    mime_type: "application/pdf".to_string(),
    transfer_id: message::TransferId::new(),
    dangerous: false,
    file_hash: [0u8; 32],
  }));
  let preview = preview_for(&msg);
  assert!(preview.starts_with("[File]"));
  assert!(preview.contains("report.pdf"));
}

#[test]
fn preview_for_forwarded() {
  let msg = make_msg(MessageContent::Forwarded {
    original_sender: UserId::from(2u64),
    content: "forwarded text".to_string(),
  });
  let preview = preview_for(&msg);
  assert!(preview.starts_with("[Forwarded]"));
  assert!(preview.contains("forwarded text"));
}

#[test]
fn preview_for_revoked() {
  let msg = make_msg(MessageContent::Revoked);
  assert_eq!(preview_for(&msg), "[Revoked]");
}

// ── Inner::process_ack_ticks ─────────────────────────────────────

#[test]
fn process_ack_ticks_returns_empty_on_idle() {
  let mut inner = Inner::new();
  let (retries, expired) = inner.process_ack_ticks(1_000);
  assert!(retries.is_empty());
  assert!(expired.is_empty());
}

#[test]
fn process_ack_ticks_extracts_expired_after_72h() {
  let mut inner = Inner::new();
  let id = MessageId::new();

  // Track a message in the ACK queue. Its created_ms is set to "now"
  // by AckQueue::track, so we test with a future timestamp that is
  // 72h + 1ms ahead of the current wall-clock time.
  inner
    .ack_queue
    .track(id, "conv".to_string(), vec![UserId::from(1u64)]);

  // Use a timestamp far enough in the future to exceed the 72-hour
  // ACK expiry window.
  let now_ms =
    chrono::Utc::now().timestamp_millis() + crate::chat::ack_queue::config::ACK_EXPIRY_MS + 1;

  let (_retries, expired) = inner.process_ack_ticks(now_ms);

  // The entry should be classified as Expired.
  assert!(
    expired.contains(&id),
    "Entry past 72h expiry should be in the expired list"
  );
}

#[test]
fn process_ack_ticks_removes_retry_payload_on_expiry() {
  let mut inner = Inner::new();
  let id = MessageId::new();
  let conv = ConversationId::Direct(UserId::from(2u64));
  let wire = DataChannelMessage::ChatText(message::datachannel::ChatText {
    message_id: id,
    content: "hello".to_string(),
    reply_to: None,
    timestamp_nanos: 0,
  });

  // Set up index and retry payload for the entry.
  inner.index.insert(id, conv);
  inner.retry_payloads.insert(id, wire);

  // Track in the ACK queue.
  inner
    .ack_queue
    .track(id, "conv".to_string(), vec![UserId::from(1u64)]);

  // Advance time past the 72h expiry.
  let now_ms =
    chrono::Utc::now().timestamp_millis() + crate::chat::ack_queue::config::ACK_EXPIRY_MS + 1;

  let (_retries, expired) = inner.process_ack_ticks(now_ms);

  // Entry should be expired and its retry_payload cleaned up.
  assert!(expired.contains(&id));
  assert!(
    !inner.retry_payloads.contains_key(&id),
    "Expired entry payload should be removed"
  );
}

#[test]
fn process_ack_ticks_skips_retry_without_index() {
  let mut inner = Inner::new();
  let id = MessageId::new();

  // Track in ACK queue but do NOT add to index.
  inner
    .ack_queue
    .track(id, "conv".to_string(), vec![UserId::from(1u64)]);

  // The tick may return Retry for this entry, but without an index
  // entry the retry is silently skipped.
  let base_time = chrono::Utc::now().timestamp_millis();
  let retry_time = base_time + crate::chat::ack_queue::config::INITIAL_BACKOFF_MS + 1;
  let (retries, _expired) = inner.process_ack_ticks(retry_time);

  // No retry should be emitted because the index is missing.
  for (_, retry_id, _) in &retries {
    assert_ne!(*retry_id, id, "Retry without index should be skipped");
  }
}

// ── Inner::expire_stale_typing ───────────────────────────────────

#[test]
fn expire_stale_typing_removes_old_entries() {
  let mut inner = Inner::new();
  let conv = ConversationId::Direct(UserId::from(1u64));
  let peer = UserId::from(42u64);

  inner.typing_peer_at.insert(
    (conv.clone(), peer.clone()),
    (0, "Alice".to_string()), // timestamp = 0 (very old)
  );

  inner.expire_stale_typing(TYPING_TIMEOUT_MS + 1);

  assert!(
    inner.typing_peer_at.is_empty(),
    "Stale typing entry should be removed"
  );
}

#[test]
fn expire_stale_typing_keeps_recent_entries() {
  let mut inner = Inner::new();
  let conv = ConversationId::Direct(UserId::from(2u64));
  let peer = UserId::from(99u64);
  let now: i64 = 10_000;

  inner.typing_peer_at.insert(
    (conv.clone(), peer.clone()),
    (now, "Bob".to_string()), // just now — not stale
  );

  inner.expire_stale_typing(now + 1);

  assert_eq!(
    inner.typing_peer_at.len(),
    1,
    "Recent typing entry should be kept"
  );
}

#[test]
fn expire_stale_typing_boundary() {
  let mut inner = Inner::new();
  let conv = ConversationId::Direct(UserId::from(3u64));
  let peer = UserId::from(7u64);

  // Insert an entry exactly at the boundary (now - TYPING_TIMEOUT_MS).
  let now: i64 = 100_000;
  inner.typing_peer_at.insert(
    (conv.clone(), peer.clone()),
    (now - TYPING_TIMEOUT_MS, "Carol".to_string()),
  );

  // At exactly the timeout, the entry should be KEPT (> check, not >=).
  inner.expire_stale_typing(now);
  assert!(
    !inner.typing_peer_at.is_empty(),
    "Entry at exact boundary should be kept (strict >, not >=)"
  );

  // One ms past the boundary, it should be removed.
  inner.expire_stale_typing(now + 1);
  assert!(
    inner.typing_peer_at.is_empty(),
    "Entry past the boundary should be removed"
  );
}

#[test]
fn expire_stale_typing_preserves_entry_within_timeout() {
  let mut inner = Inner::new();
  let conv = ConversationId::Direct(UserId::from(4u64));
  let peer = UserId::from(13u64);
  let now: i64 = 200_000;

  // 1 ms before the timeout — should survive.
  inner.typing_peer_at.insert(
    (conv.clone(), peer.clone()),
    (now - TYPING_TIMEOUT_MS + 1, "Dave".to_string()),
  );

  inner.expire_stale_typing(now);
  assert_eq!(inner.typing_peer_at.len(), 1);
}

// ── Inner::new ───────────────────────────────────────────────────

#[test]
fn inner_new_has_empty_collections() {
  let inner = Inner::new();
  assert!(inner.conversations.is_empty());
  assert!(inner.index.is_empty());
  assert!(inner.retry_payloads.is_empty());
  assert!(inner.typing_sent_at.is_empty());
  assert!(inner.typing_peer_at.is_empty());
}

// ── Typing rate-limit constants ──────────────────────────────────

#[test]
fn typing_timeout_is_five_seconds() {
  assert_eq!(TYPING_TIMEOUT_MS, 5_000);
}

#[test]
fn typing_rate_limit_is_three_seconds() {
  assert_eq!(TYPING_RATE_LIMIT_MS, 3_000);
}

#[test]
fn rate_limit_shorter_than_timeout() {
  // The rate limit must be strictly shorter than the timeout so a user
  // can send multiple typing indicators before the indicator disappears.
  const { assert!(TYPING_RATE_LIMIT_MS < TYPING_TIMEOUT_MS) };
}
