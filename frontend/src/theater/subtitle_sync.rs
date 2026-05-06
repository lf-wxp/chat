//! Theater subtitle sync helpers (Req 12.4a).
//!
//! These functions mutate [`TheaterState`] in response to three
//! events:
//!
//! 1. Owner finished parsing a subtitle file → [`apply_subtitle_track`]
//!    populates `state.subtitle` locally.
//! 2. Owner broadcast a `SubtitleData` frame → [`apply_subtitle_data`]
//!    writes the same payload into viewer state.
//! 3. Owner broadcast a `SubtitleClear` frame → [`apply_subtitle_clear`]
//!    wipes the viewer track.
//!
//! The playback loop also calls [`refresh_active_subtitle`] on every
//! `timeupdate` tick so the overlay component can render the current
//! cue without scanning the list itself.
//!
//! Signal-touching helpers are thin wrappers around pure helpers that
//! live alongside unit tests; the wrappers themselves require a
//! Leptos `Owner` and therefore are only exercised from the browser
//! runtime.

use leptos::prelude::*;
use message::datachannel::{SubtitleClear, SubtitleData};
use message::types::RoomId;

use super::state::{SubtitleTrack, TheaterState};
use super::subtitle::active_entry;

/// Pure helper: turn a `SubtitleData` payload into a [`SubtitleTrack`]
/// when the frame targets the active room. Returns `None` when the
/// room does not match.
#[must_use]
pub fn build_track_from_data(
  active_room: Option<&RoomId>,
  payload: &SubtitleData,
) -> Option<SubtitleTrack> {
  if active_room != Some(&payload.room_id) {
    return None;
  }
  Some(SubtitleTrack {
    filename: String::new(),
    entries: payload.entries.clone(),
    visible: true,
  })
}

/// Pure helper: decide whether an inbound `SubtitleClear` applies to
/// the active room.
#[must_use]
pub fn should_apply_clear(active_room: Option<&RoomId>, payload: &SubtitleClear) -> bool {
  active_room == Some(&payload.room_id)
}

/// Pure helper: pick the active subtitle text for a given playback
/// timestamp, respecting the `visible` flag.
#[must_use]
pub fn pick_active_text(track: Option<&SubtitleTrack>, time_ms: u32) -> Option<String> {
  let track = track?;
  if !track.visible {
    return None;
  }
  active_entry(&track.entries, time_ms).map(|entry| entry.text.clone())
}

/// Install a freshly parsed subtitle track on the local state.
///
/// Also clears the currently rendered cue so the overlay refreshes on
/// the next `timeupdate` tick.
pub fn apply_subtitle_track(state: &TheaterState, track: SubtitleTrack) {
  state.subtitle.set(Some(track));
  state.active_subtitle_text.set(None);
}

/// Apply an inbound `SubtitleData` frame to the viewer state. No-op
/// (returns `false`) when the frame targets a different room.
pub fn apply_subtitle_data(state: &TheaterState, msg: SubtitleData) -> bool {
  let active = state.room_id.get_untracked();
  let Some(track) = build_track_from_data(active.as_ref(), &msg) else {
    return false;
  };
  apply_subtitle_track(state, track);
  true
}

/// Apply an inbound `SubtitleClear` frame to the viewer state.
pub fn apply_subtitle_clear(state: &TheaterState, msg: &SubtitleClear) -> bool {
  let active = state.room_id.get_untracked();
  if !should_apply_clear(active.as_ref(), msg) {
    return false;
  }
  state.subtitle.set(None);
  state.active_subtitle_text.set(None);
  true
}

/// Update [`TheaterState::active_subtitle_text`] for the current
/// playback timestamp. Invoked from the video player's timeupdate
/// handler so the overlay renders synchronously with the video.
///
/// Returns `true` when the active text actually changed.
pub fn refresh_active_subtitle(state: &TheaterState, time_ms: u32) -> bool {
  let next = state
    .subtitle
    .with_untracked(|track| pick_active_text(track.as_ref(), time_ms));
  let prev = state.active_subtitle_text.get_untracked();
  if prev != next {
    state.active_subtitle_text.set(next);
    true
  } else {
    false
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use message::datachannel::SubtitleEntry;
  use uuid::Uuid;

  fn mk_room() -> RoomId {
    RoomId(Uuid::from_u128(42))
  }

  fn entry(start_ms: u32, end_ms: u32, text: &str) -> SubtitleEntry {
    SubtitleEntry {
      start_ms,
      end_ms,
      text: text.into(),
    }
  }

  #[test]
  fn build_track_accepts_matching_room() {
    let room = mk_room();
    let payload = SubtitleData {
      room_id: room.clone(),
      entries: vec![entry(0, 1_000, "Hello")],
    };
    let track = build_track_from_data(Some(&room), &payload).expect("track");
    assert!(track.visible);
    assert_eq!(track.entries.len(), 1);
  }

  #[test]
  fn build_track_rejects_other_rooms() {
    let room = mk_room();
    let other = RoomId(Uuid::from_u128(99));
    let payload = SubtitleData {
      room_id: other,
      entries: vec![entry(0, 1_000, "Hello")],
    };
    assert!(build_track_from_data(Some(&room), &payload).is_none());
    assert!(build_track_from_data(None, &payload).is_none());
  }

  #[test]
  fn clear_only_applies_to_active_room() {
    let room = mk_room();
    let other = RoomId(Uuid::from_u128(99));
    assert!(should_apply_clear(
      Some(&room),
      &SubtitleClear {
        room_id: room.clone()
      }
    ));
    assert!(!should_apply_clear(
      Some(&room),
      &SubtitleClear { room_id: other }
    ));
    assert!(!should_apply_clear(None, &SubtitleClear { room_id: room }));
  }

  #[test]
  fn pick_active_text_respects_visibility() {
    let track = SubtitleTrack {
      filename: "m.srt".into(),
      entries: vec![entry(0, 1_000, "Hello")],
      visible: false,
    };
    assert_eq!(pick_active_text(Some(&track), 500), None);
  }

  #[test]
  fn pick_active_text_returns_current_cue() {
    let track = SubtitleTrack {
      filename: "m.srt".into(),
      entries: vec![entry(0, 1_000, "Hello"), entry(1_000, 2_000, "World")],
      visible: true,
    };
    assert_eq!(
      pick_active_text(Some(&track), 500).as_deref(),
      Some("Hello")
    );
    assert_eq!(
      pick_active_text(Some(&track), 1_500).as_deref(),
      Some("World")
    );
    assert_eq!(pick_active_text(Some(&track), 3_000), None);
  }

  #[test]
  fn pick_active_text_none_without_track() {
    assert_eq!(pick_active_text(None, 500), None);
  }
}
