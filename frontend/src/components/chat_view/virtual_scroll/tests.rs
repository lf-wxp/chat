//! Unit tests for `estimate_height`, `compute_window`, and `total_height`.

use super::*;
use crate::chat::models::{
  ChatMessage, ImageRef, MessageContent, MessageStatus, StickerRef, VoiceClip,
};
use message::{MessageId, UserId};
use std::collections::BTreeMap;

/// Helper: build a minimal `ChatMessage` with given content.
fn make_msg(content: MessageContent) -> ChatMessage {
  ChatMessage {
    id: MessageId::new(),
    sender: UserId::from(1u64),
    sender_name: "Test".to_string(),
    content,
    timestamp_ms: 0,
    outgoing: false,
    status: MessageStatus::Received,
    reply_to: None,
    read_by: Vec::new(),
    reactions: BTreeMap::new(),
    mentions_me: false,
    counted_unread: false,
  }
}

fn text_msg(s: &str) -> ChatMessage {
  make_msg(MessageContent::Text(s.to_string()))
}

fn sticker_msg() -> ChatMessage {
  make_msg(MessageContent::Sticker(StickerRef {
    pack_id: "smileys".into(),
    sticker_id: "grin".into(),
  }))
}

fn voice_msg() -> ChatMessage {
  make_msg(MessageContent::Voice(VoiceClip {
    object_url: "blob:test".into(),
    duration_ms: 5000,
    waveform: vec![128; 20],
  }))
}

fn image_msg(w: u32, h: u32) -> ChatMessage {
  make_msg(MessageContent::Image(ImageRef {
    object_url: "blob:test".into(),
    thumbnail_url: "blob:test-thumb".into(),
    width: w,
    height: h,
  }))
}

fn forwarded_msg(content: &str) -> ChatMessage {
  make_msg(MessageContent::Forwarded {
    original_sender: UserId::from(2u64),
    content: content.to_string(),
  })
}

fn revoked_msg() -> ChatMessage {
  make_msg(MessageContent::Revoked)
}

// ── estimate_height tests ─────────────────────────────────

#[test]
fn estimate_height_short_text_single_line() {
  // Short ASCII text → 1 line → 48 px base.
  let msg = text_msg("Hello!");
  assert_eq!(estimate_height(&msg), 48.0);
}

#[test]
fn estimate_height_text_wraps() {
  // 80 chars → ceil(80/40) = 2 lines → 48 + 20 = 68.
  let long = "a".repeat(80);
  let msg = text_msg(&long);
  assert_eq!(estimate_height(&msg), 68.0);
}

#[test]
fn estimate_height_cjk_text() {
  // 40 CJK characters → exactly 1 line → 48 px.
  let cjk = "你".repeat(40);
  let msg = text_msg(&cjk);
  assert_eq!(estimate_height(&msg), 48.0);
}

#[test]
fn estimate_height_cjk_multi_line() {
  // 120 CJK characters → ceil(120/40) = 3 lines → 48 + 2*20 = 88.
  let cjk = "你".repeat(120);
  let msg = text_msg(&cjk);
  assert_eq!(estimate_height(&msg), 88.0);
}

#[test]
fn estimate_height_empty_text() {
  // Empty string → max(1.0) = 1 line → 48.
  let msg = text_msg("");
  assert_eq!(estimate_height(&msg), 48.0);
}

#[test]
fn estimate_height_sticker_is_120() {
  let msg = sticker_msg();
  assert_eq!(estimate_height(&msg), 120.0);
}

#[test]
fn estimate_height_voice_is_60() {
  let msg = voice_msg();
  assert_eq!(estimate_height(&msg), 60.0);
}

#[test]
fn estimate_height_revoked_is_48() {
  let msg = revoked_msg();
  assert_eq!(estimate_height(&msg), 48.0);
}

#[test]
fn estimate_height_image_square() {
  // 300x300 → aspect=1.0, height=300*1+32=332.
  let msg = image_msg(300, 300);
  assert_eq!(estimate_height(&msg), 332.0);
}

#[test]
fn estimate_height_image_tall_clamped() {
  // 100x2000 → aspect=20.0, 300*20=6000 → clamped to 400 + 32 = 432.
  let msg = image_msg(100, 2000);
  assert_eq!(estimate_height(&msg), 432.0);
}

#[test]
fn estimate_height_image_small_clamped() {
  // 1000x10 → aspect=0.01, 300*0.01=3 → clamped to 60 + 32 = 92.
  let msg = image_msg(1000, 10);
  assert_eq!(estimate_height(&msg), 92.0);
}

#[test]
fn estimate_height_forwarded_single_line() {
  let msg = forwarded_msg("short");
  // 1 line → 72 px base.
  assert_eq!(estimate_height(&msg), 72.0);
}

#[test]
fn estimate_height_forwarded_multi_line() {
  // 80 chars → 2 lines → 72 + 20 = 92.
  let long = "b".repeat(80);
  let msg = forwarded_msg(&long);
  assert_eq!(estimate_height(&msg), 92.0);
}

// ── compute_window tests ──────────────────────────────────

fn make_n_text_msgs(n: usize) -> Vec<ChatMessage> {
  (0..n).map(|_| text_msg("hello")).collect()
}

#[test]
fn compute_window_empty_list() {
  let cache = HeightCache::default();
  let (start, end, offset) = compute_window(&[], &cache, 0.0, 600.0);
  assert_eq!(start, 0);
  assert_eq!(end, 0);
  assert_eq!(offset, 0.0);
}

#[test]
fn compute_window_all_visible() {
  // 5 messages × 48px = 240px, viewport 600px → all visible.
  let msgs = make_n_text_msgs(5);
  let cache = HeightCache::default();
  let (start, end, offset) = compute_window(&msgs, &cache, 0.0, 600.0);
  assert_eq!(start, 0);
  assert_eq!(end, 5); // all items (5 + OVERSCAN=3 clamped to len)
  assert_eq!(offset, 0.0);
}

#[test]
fn compute_window_scrolled_midway() {
  // 20 messages × 48px = 960px total. Viewport 200px.
  // scroll_top = 200 → row 4 starts at 192, row 5 at 240.
  // First visible ≈ index 4 (accumulated=192, 192+48>200).
  // Viewport fills ~4 rows (200/48≈4.2), so end ≈ 9.
  // With overscan: start=max(0,4-3)=1, end=min(20,9+3)=12.
  let msgs = make_n_text_msgs(20);
  let cache = HeightCache::default();
  let (start, end, offset) = compute_window(&msgs, &cache, 200.0, 200.0);

  // Start should be backed by overscan from visible start.
  assert!(start <= 4);
  // End should cover beyond visible end + overscan.
  assert!(end >= 8);
  assert!(end <= 20);
  // Offset should equal sum of heights before `start`.
  let expected_offset: f64 = (0..start).map(|_| 48.0).sum();
  assert_eq!(offset, expected_offset);
}

#[test]
fn compute_window_scrolled_to_bottom() {
  // 20 messages × 48px = 960px total. Viewport 200px.
  // scroll_top = 760 → last visible starts at row 15 (15*48=720).
  let msgs = make_n_text_msgs(20);
  let cache = HeightCache::default();
  let (start, end, _) = compute_window(&msgs, &cache, 760.0, 200.0);

  // End should be 20 (at the list boundary).
  assert_eq!(end, 20);
  // Start should be backed off by overscan from the last visible rows.
  assert!(start >= 10);
  assert!(start <= 17);
}

#[test]
fn compute_window_uses_cached_heights() {
  // 10 messages: cache half as 100px each, rest default 48px.
  let msgs = make_n_text_msgs(10);
  let cache = HeightCache::default();
  for msg in &msgs[..5] {
    cache.insert(msg.id, 100.0);
  }
  // Total = 5*100 + 5*48 = 740px.
  // Viewport = 200px, scrollTop = 0 → first row visible is 0.
  let (start, end, offset) = compute_window(&msgs, &cache, 0.0, 200.0);
  assert_eq!(start, 0);
  assert_eq!(offset, 0.0);
  // 200px viewport filled by 2 rows of 100px → end ≈ 2, + overscan 3 → 5.
  assert!((2..=8).contains(&end));
}

#[test]
fn compute_window_overscan_is_3() {
  // Verify the overscan constant. With 30 messages and viewport in
  // the middle, there should be exactly 3 items of overscan.
  let msgs = make_n_text_msgs(30);
  let cache = HeightCache::default();
  // scroll_top = 480 → first visible row at index 10 (10*48=480).
  // Viewport = 192px → fits 4 rows → visible end = 14.
  let (start, end, _) = compute_window(&msgs, &cache, 480.0, 192.0);
  // With OVERSCAN=3: start = 10-3 = 7, end = 14+3 = 17.
  assert_eq!(start, 7);
  assert_eq!(end, 17);
}

// ── total_height tests ────────────────────────────────────

#[test]
fn total_height_homogeneous() {
  let msgs = make_n_text_msgs(10);
  let cache = HeightCache::default();
  assert_eq!(total_height(&msgs, &cache), 480.0); // 10 * 48.0
}

#[test]
fn total_height_mixed_content() {
  let msgs = vec![text_msg("hi"), sticker_msg(), voice_msg()];
  let cache = HeightCache::default();
  // 48 + 120 + 60 = 228.
  assert_eq!(total_height(&msgs, &cache), 228.0);
}

#[test]
fn total_height_respects_cache() {
  let msgs = vec![text_msg("hi")];
  let cache = HeightCache::default();
  cache.insert(msgs[0].id, 999.0);
  assert_eq!(total_height(&msgs, &cache), 999.0);
}

// ── HeightCache tests ─────────────────────────────────────

#[test]
fn height_cache_basic_insert_get() {
  let cache = HeightCache::default();
  let id = MessageId::new();
  cache.insert(id, 100.0);
  assert_eq!(cache.get(&id), Some(100.0));
  assert_eq!(cache.len(), 1);
}

#[test]
fn height_cache_clear() {
  let cache = HeightCache::default();
  for _ in 0..10 {
    cache.insert(MessageId::new(), 100.0);
  }
  assert_eq!(cache.len(), 10);
  cache.clear();
  assert!(cache.is_empty());
}

#[test]
fn height_cache_evicts_when_full() {
  // Insert more than HEIGHT_CACHE_MAX_SIZE entries to trigger eviction.
  // HEIGHT_CACHE_MAX_SIZE is 2000, so we insert 2100 entries.
  let cache = HeightCache::default();
  let ids: Vec<_> = (0..2100).map(|_| MessageId::new()).collect();

  for id in &ids {
    cache.insert(*id, 100.0);
  }

  // After eviction, the cache should be reduced (half evicted).
  // 2100 > 2000 triggers eviction, removing ~1050 entries → ~1050 remain.
  assert!(cache.len() < 2100);
  assert!(cache.len() <= super::HEIGHT_CACHE_MAX_SIZE);
}
