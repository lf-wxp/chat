//! Native unit tests for the theater DC router.
//!
//! Only pure functions (`classify`, `should_dispatch`) are exercised
//! here — the `apply` path interacts with `TheaterState` which in
//! turn touches `web_sys::window()`, so `apply` is covered by the
//! `wasm-bindgen-test` suite in a separate batch.

use message::MessageId;
use message::UserId;
use message::datachannel::{
  ChatText, Danmaku, DanmakuBatch, DataChannelMessage, PlaybackProgress, SubtitleClear,
  SubtitleData, TheaterChatText,
};
use message::types::{DanmakuPosition, RoomId, SubtitleEntry};
use uuid::Uuid;

use super::{TheaterInbound, classify, should_dispatch};

fn make_room_id(seed: u128) -> RoomId {
  RoomId::from_uuid(Uuid::from_u128(seed))
}

fn make_danmaku() -> Danmaku {
  Danmaku {
    content: "hi".into(),
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
    entries: vec![SubtitleEntry {
      start_ms: 0,
      end_ms: 1_000,
      text: "subtitle".into(),
    }],
  }
}

fn make_subtitle_clear(room: RoomId) -> SubtitleClear {
  SubtitleClear { room_id: room }
}

fn make_playback(room: RoomId) -> PlaybackProgress {
  PlaybackProgress {
    room_id: room,
    current_time_ms: 1_000,
    duration_ms: 60_000,
    is_paused: false,
    timestamp_nanos: 0,
  }
}

#[test]
fn classify_recognises_theater_variants() {
  assert!(matches!(
    classify(DataChannelMessage::Danmaku(make_danmaku())),
    Ok(TheaterInbound::Danmaku(_))
  ));
  let room = make_room_id(1);
  assert!(matches!(
    classify(DataChannelMessage::SubtitleData(make_subtitle_data(
      room.clone()
    ))),
    Ok(TheaterInbound::SubtitleData(_))
  ));
  assert!(matches!(
    classify(DataChannelMessage::SubtitleClear(make_subtitle_clear(
      room.clone()
    ))),
    Ok(TheaterInbound::SubtitleClear(_))
  ));
  assert!(matches!(
    classify(DataChannelMessage::PlaybackProgress(make_playback(
      room.clone()
    ))),
    Ok(TheaterInbound::Playback(_))
  ));
  assert!(matches!(
    classify(DataChannelMessage::DanmakuBatch(DanmakuBatch {
      room_id: room.clone(),
      entries: vec![make_danmaku()],
    })),
    Ok(TheaterInbound::DanmakuBatch(_))
  ));
  assert!(matches!(
    classify(DataChannelMessage::TheaterChatText(TheaterChatText {
      room_id: room,
      sender_id: UserId::default(),
      content: String::new(),
      timestamp_nanos: 0,
    })),
    Ok(TheaterInbound::Chat(_))
  ));
}

#[test]
fn classify_rejects_non_theater_variants() {
  let chat = DataChannelMessage::ChatText(ChatText {
    message_id: MessageId::new(),
    content: "hi".into(),
    reply_to: None,
    timestamp_nanos: 0,
  });
  assert!(classify(chat).is_err());
}

#[test]
fn classify_round_trips_err_payload() {
  // The boxed `DataChannelMessage` round-trips unchanged so the
  // caller can continue inspecting the original value after a
  // missed classification.
  let original = DataChannelMessage::ChatText(ChatText {
    message_id: MessageId::nil(),
    content: "hi".into(),
    reply_to: None,
    timestamp_nanos: 0,
  });
  let returned = classify(original.clone()).expect_err("non-theater variant");
  assert_eq!(*returned, original);
}

#[test]
fn should_dispatch_accepts_danmaku_for_any_active_room() {
  let room = make_room_id(2);
  let inbound = TheaterInbound::Danmaku(make_danmaku());
  assert!(should_dispatch(Some(&room), &inbound));
}

#[test]
fn should_dispatch_rejects_when_not_in_a_room() {
  let inbound = TheaterInbound::Danmaku(make_danmaku());
  assert!(!should_dispatch(None, &inbound));
}

#[test]
fn should_dispatch_filters_foreign_room_messages() {
  let mine = make_room_id(3);
  let other = make_room_id(4);
  let inbound = TheaterInbound::SubtitleData(make_subtitle_data(other));
  assert!(!should_dispatch(Some(&mine), &inbound));
}

#[test]
fn should_dispatch_accepts_matching_room_messages() {
  let mine = make_room_id(5);
  let inbound = TheaterInbound::Playback(make_playback(mine.clone()));
  assert!(should_dispatch(Some(&mine), &inbound));
}

#[test]
fn inbound_room_id_matches_payload_room() {
  let room = make_room_id(6);
  let subtitle = TheaterInbound::SubtitleData(make_subtitle_data(room.clone()));
  assert_eq!(subtitle.room_id(), Some(&room));
  let danmaku = TheaterInbound::Danmaku(make_danmaku());
  assert_eq!(danmaku.room_id(), None);
  let batch = TheaterInbound::DanmakuBatch(DanmakuBatch {
    room_id: room.clone(),
    entries: vec![],
  });
  assert_eq!(batch.room_id(), Some(&room));
  let chat = TheaterInbound::Chat(TheaterChatText {
    room_id: room.clone(),
    sender_id: UserId::default(),
    content: String::new(),
    timestamp_nanos: 0,
  });
  assert_eq!(chat.room_id(), Some(&room));
}
