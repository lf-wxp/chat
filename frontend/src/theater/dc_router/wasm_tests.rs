//! WASM integration tests for `dc_router::apply`.
//!
//! These tests exercise the full `apply` path which requires a live
//! Leptos reactive runtime and `web_sys::window()` access (for
//! localStorage-backed overlay settings). They complement the native
//! unit tests in `tests.rs` which only cover the pure `classify` and
//! `should_dispatch` helpers.

use std::collections::VecDeque;

use leptos::prelude::{GetUntracked, Set};
use message::UserId;
use message::datachannel::{
  Danmaku, DanmakuBatch, PlaybackProgress, SubtitleClear, SubtitleData, TheaterChatText,
};
use message::types::{DanmakuPosition, RoomId, SubtitleEntry};
use uuid::Uuid;
use wasm_bindgen_test::*;

use super::{TheaterInbound, apply};
use crate::theater::state::{TheaterRole, TheaterState};

wasm_bindgen_test_configure!(run_in_browser);

/// Test wrapper that calls `apply` with a fallback name resolver
/// (just returns the user id as a string). Production code passes a
/// closure that looks up the online-users list.
fn apply_test(state: &TheaterState, inbound: TheaterInbound) -> bool {
  apply(state, inbound, |id| id.to_string())
}

fn make_room_id(seed: u128) -> RoomId {
  RoomId::from_uuid(Uuid::from_u128(seed))
}

fn make_danmaku(content: &str) -> Danmaku {
  Danmaku {
    content: content.into(),
    font_size: 24,
    color: 0x00FF_FFFF,
    position: DanmakuPosition::Scroll,
    video_time_ms: 0,
    timestamp_nanos: 0,
  }
}

fn make_subtitle_data(room: RoomId) -> SubtitleData {
  SubtitleData {
    room_id: room,
    entries: vec![
      SubtitleEntry {
        start_ms: 0,
        end_ms: 2_000,
        text: "Hello world".into(),
      },
      SubtitleEntry {
        start_ms: 2_000,
        end_ms: 4_000,
        text: "Second cue".into(),
      },
    ],
  }
}

fn make_subtitle_clear(room: RoomId) -> SubtitleClear {
  SubtitleClear { room_id: room }
}

fn make_playback(room: RoomId, time_ms: u64) -> PlaybackProgress {
  PlaybackProgress {
    room_id: room,
    current_time_ms: time_ms,
    duration_ms: 120_000,
    is_paused: false,
    timestamp_nanos: 0,
  }
}

// ── Danmaku apply tests ─────────────────────────────────────────────

#[wasm_bindgen_test]
fn apply_danmaku_pushes_to_incoming_queue() {
  let state = TheaterState::new();
  let room = make_room_id(100);
  state.room_id.set(Some(room));

  let d = make_danmaku("test danmaku");
  let accepted = apply_test(&state, TheaterInbound::Danmaku(d.clone()));

  assert!(accepted);
  let queue: VecDeque<Danmaku> = state.incoming_danmaku.get_untracked();
  assert_eq!(queue.len(), 1);
  assert_eq!(queue[0].content, "test danmaku");
}

#[wasm_bindgen_test]
fn apply_danmaku_rejected_when_no_active_room() {
  let state = TheaterState::new();
  // room_id is None by default.

  let d = make_danmaku("orphan");
  let accepted = apply_test(&state, TheaterInbound::Danmaku(d));

  assert!(!accepted);
  let queue: VecDeque<Danmaku> = state.incoming_danmaku.get_untracked();
  assert!(queue.is_empty());
}

#[wasm_bindgen_test]
fn apply_multiple_danmaku_accumulates_in_order() {
  let state = TheaterState::new();
  let room = make_room_id(101);
  state.room_id.set(Some(room));

  apply_test(&state, TheaterInbound::Danmaku(make_danmaku("first")));
  apply_test(&state, TheaterInbound::Danmaku(make_danmaku("second")));
  apply_test(&state, TheaterInbound::Danmaku(make_danmaku("third")));

  let queue: VecDeque<Danmaku> = state.incoming_danmaku.get_untracked();
  assert_eq!(queue.len(), 3);
  assert_eq!(queue[0].content, "first");
  assert_eq!(queue[1].content, "second");
  assert_eq!(queue[2].content, "third");
}

// ── Subtitle apply tests ────────────────────────────────────────────

#[wasm_bindgen_test]
fn apply_subtitle_data_populates_track() {
  let state = TheaterState::new();
  let room = make_room_id(200);
  state.room_id.set(Some(room.clone()));

  let data = make_subtitle_data(room);
  let accepted = apply_test(&state, TheaterInbound::SubtitleData(data));

  assert!(accepted);
  let track = state.subtitle.get_untracked();
  assert!(track.is_some());
  let track = track.unwrap();
  assert_eq!(track.entries.len(), 2);
  assert_eq!(track.entries[0].text, "Hello world");
  assert_eq!(track.entries[1].text, "Second cue");
  assert!(track.visible);
}

#[wasm_bindgen_test]
fn apply_subtitle_data_rejected_for_wrong_room() {
  let state = TheaterState::new();
  let my_room = make_room_id(201);
  let other_room = make_room_id(202);
  state.room_id.set(Some(my_room));

  let data = make_subtitle_data(other_room);
  let accepted = apply_test(&state, TheaterInbound::SubtitleData(data));

  assert!(!accepted);
  assert!(state.subtitle.get_untracked().is_none());
}

#[wasm_bindgen_test]
fn apply_subtitle_clear_removes_track() {
  let state = TheaterState::new();
  let room = make_room_id(203);
  state.room_id.set(Some(room.clone()));

  // First populate a track.
  let data = make_subtitle_data(room.clone());
  apply_test(&state, TheaterInbound::SubtitleData(data));
  assert!(state.subtitle.get_untracked().is_some());

  // Then clear it.
  let clear = make_subtitle_clear(room);
  let accepted = apply_test(&state, TheaterInbound::SubtitleClear(clear));

  assert!(accepted);
  assert!(state.subtitle.get_untracked().is_none());
}

#[wasm_bindgen_test]
fn apply_subtitle_clear_rejected_for_wrong_room() {
  let state = TheaterState::new();
  let my_room = make_room_id(204);
  let other_room = make_room_id(205);
  state.room_id.set(Some(my_room.clone()));

  // Populate a track for my room.
  let data = make_subtitle_data(my_room);
  apply_test(&state, TheaterInbound::SubtitleData(data));

  // Try to clear from a different room — should be rejected.
  let clear = make_subtitle_clear(other_room);
  let accepted = apply_test(&state, TheaterInbound::SubtitleClear(clear));

  assert!(!accepted);
  // Track should still be present.
  assert!(state.subtitle.get_untracked().is_some());
}

// ── Playback apply tests ────────────────────────────────────────────

#[wasm_bindgen_test]
fn apply_playback_updates_snapshot() {
  let state = TheaterState::new();
  let room = make_room_id(300);
  state.room_id.set(Some(room.clone()));

  let progress = make_playback(room, 5_000);
  let accepted = apply_test(&state, TheaterInbound::Playback(progress));

  assert!(accepted);
  let snap = state.playback.get_untracked();
  assert_eq!(snap.current_time_ms, 5_000);
  assert_eq!(snap.duration_ms, 120_000);
  assert!(!snap.is_paused);
}

#[wasm_bindgen_test]
fn apply_playback_rejected_for_wrong_room() {
  let state = TheaterState::new();
  let my_room = make_room_id(301);
  let other_room = make_room_id(302);
  state.room_id.set(Some(my_room));

  let progress = make_playback(other_room, 10_000);
  let accepted = apply_test(&state, TheaterInbound::Playback(progress));

  assert!(!accepted);
  // Playback should remain at default (0).
  let snap = state.playback.get_untracked();
  assert_eq!(snap.current_time_ms, 0);
}

#[wasm_bindgen_test]
fn apply_playback_idempotent_for_same_values() {
  let state = TheaterState::new();
  let room = make_room_id(303);
  state.room_id.set(Some(room.clone()));

  let progress = make_playback(room.clone(), 7_000);
  let first = apply_test(&state, TheaterInbound::Playback(progress.clone()));
  assert!(first);

  // Applying the same progress again should return false (no change).
  let second = apply_test(&state, TheaterInbound::Playback(progress));
  assert!(!second);
}

// ── Mixed scenario tests ────────────────────────────────────────────

#[wasm_bindgen_test]
fn apply_interleaved_messages_all_dispatch_correctly() {
  let state = TheaterState::new();
  let room = make_room_id(400);
  state.room_id.set(Some(room.clone()));

  // Danmaku
  apply_test(&state, TheaterInbound::Danmaku(make_danmaku("hello")));

  // Subtitle
  apply_test(
    &state,
    TheaterInbound::SubtitleData(make_subtitle_data(room.clone())),
  );

  // Playback
  apply_test(
    &state,
    TheaterInbound::Playback(make_playback(room.clone(), 3_000)),
  );

  // Another danmaku
  apply_test(&state, TheaterInbound::Danmaku(make_danmaku("world")));

  // Verify all state was updated correctly.
  let queue: VecDeque<Danmaku> = state.incoming_danmaku.get_untracked();
  assert_eq!(queue.len(), 2);
  assert_eq!(queue[0].content, "hello");
  assert_eq!(queue[1].content, "world");

  let track = state.subtitle.get_untracked().unwrap();
  assert_eq!(track.entries.len(), 2);

  let snap = state.playback.get_untracked();
  assert_eq!(snap.current_time_ms, 3_000);
}

#[wasm_bindgen_test]
fn apply_after_room_change_filters_stale_messages() {
  let state = TheaterState::new();
  let room_a = make_room_id(500);
  let room_b = make_room_id(501);

  // Start in room A.
  state.room_id.set(Some(room_a.clone()));
  apply_test(
    &state,
    TheaterInbound::Playback(make_playback(room_a.clone(), 1_000)),
  );
  assert_eq!(state.playback.get_untracked().current_time_ms, 1_000);

  // Switch to room B.
  state.room_id.set(Some(room_b.clone()));

  // A stale message from room A should be rejected.
  let stale = apply_test(
    &state,
    TheaterInbound::Playback(make_playback(room_a, 99_000)),
  );
  assert!(!stale);
  // Playback should still show the old value (not updated by stale msg).
  // Note: playback was set to 1000 while in room A, but after room
  // switch the state is not automatically reset — that's the page's
  // responsibility. The important thing is the stale message was rejected.

  // A message for room B should be accepted.
  let fresh = apply_test(
    &state,
    TheaterInbound::Playback(make_playback(room_b, 2_000)),
  );
  assert!(fresh);
  assert_eq!(state.playback.get_untracked().current_time_ms, 2_000);
}

// ── Owner relay enqueue (C1) ────────────────────────────────────────

#[wasm_bindgen_test]
fn apply_danmaku_enqueues_on_owner_batcher() {
  let state = TheaterState::new();
  let room = make_room_id(600);
  state.room_id.set(Some(room));
  state.my_role.set(TheaterRole::Owner);

  let d = make_danmaku("from viewer");
  assert!(apply_test(&state, TheaterInbound::Danmaku(d.clone())));

  // Overlay gets a copy.
  let queue: VecDeque<Danmaku> = state.incoming_danmaku.get_untracked();
  assert_eq!(queue.len(), 1);

  // Relay batcher also has it, ready for the 50 ms tick to drain.
  let pending = state.with_danmaku_batcher::<usize>(|b| b.pending_len());
  assert_eq!(pending, 1);
}

#[wasm_bindgen_test]
fn apply_danmaku_does_not_enqueue_for_viewers() {
  let state = TheaterState::new();
  let room = make_room_id(601);
  state.room_id.set(Some(room));
  state.my_role.set(TheaterRole::Viewer);

  apply_test(&state, TheaterInbound::Danmaku(make_danmaku("x")));

  let pending = state.with_danmaku_batcher::<usize>(|b| b.pending_len());
  assert_eq!(pending, 0);
}

// ── DanmakuBatch splatting (I1) ─────────────────────────────────────

#[wasm_bindgen_test]
fn apply_danmaku_batch_splats_into_overlay_queue() {
  let state = TheaterState::new();
  let room = make_room_id(700);
  state.room_id.set(Some(room.clone()));
  state.my_role.set(TheaterRole::Viewer);

  let batch = DanmakuBatch {
    room_id: room,
    entries: vec![make_danmaku("a"), make_danmaku("b"), make_danmaku("c")],
  };
  assert!(apply_test(&state, TheaterInbound::DanmakuBatch(batch)));

  let queue: VecDeque<Danmaku> = state.incoming_danmaku.get_untracked();
  assert_eq!(queue.len(), 3);
  assert_eq!(queue[0].content, "a");
  assert_eq!(queue[2].content, "c");
}

#[wasm_bindgen_test]
fn apply_danmaku_batch_rejected_on_owner() {
  // Owners never receive their own fan-out back; guard ensures a
  // misrouted batch cannot trigger an infinite relay loop.
  let state = TheaterState::new();
  let room = make_room_id(701);
  state.room_id.set(Some(room.clone()));
  state.my_role.set(TheaterRole::Owner);

  let batch = DanmakuBatch {
    room_id: room,
    entries: vec![make_danmaku("loopback")],
  };
  assert!(!apply_test(&state, TheaterInbound::DanmakuBatch(batch)));
  assert!(state.incoming_danmaku.get_untracked().is_empty());
}

// ── TheaterChatText routing (C2) ────────────────────────────────────

#[wasm_bindgen_test]
fn apply_chat_appends_to_local_log_for_viewer() {
  let state = TheaterState::new();
  let room = make_room_id(800);
  state.room_id.set(Some(room.clone()));
  state.my_role.set(TheaterRole::Viewer);

  let payload = TheaterChatText {
    room_id: room,
    sender_id: UserId::default(),
    content: "hello viewers".into(),
    timestamp_nanos: 1_000_000_000,
  };
  assert!(apply_test(&state, TheaterInbound::Chat(payload)));

  let msgs = state.chat_messages.get_untracked();
  assert_eq!(msgs.len(), 1);
  assert_eq!(msgs[0].content, "hello viewers");
  assert!(!msgs[0].is_self);

  // Viewers do not relay — queue must stay empty.
  let relay = state.pending_chat_relay.get_untracked();
  assert!(relay.is_empty());
}

#[wasm_bindgen_test]
fn apply_chat_queues_relay_on_owner() {
  let state = TheaterState::new();
  let room = make_room_id(801);
  state.room_id.set(Some(room.clone()));
  state.my_role.set(TheaterRole::Owner);

  let payload = TheaterChatText {
    room_id: room,
    sender_id: UserId::default(),
    content: "relay me".into(),
    timestamp_nanos: 1_000_000_000,
  };
  assert!(apply_test(&state, TheaterInbound::Chat(payload.clone())));

  // Owner appends to its own log so the bubble is visible locally.
  assert_eq!(state.chat_messages.get_untracked().len(), 1);

  // And queues the payload for the 50 ms relay tick.
  let relay = state.drain_chat_relay();
  assert_eq!(relay.len(), 1);
  assert_eq!(relay[0].content, "relay me");
}
