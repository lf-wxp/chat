//! Theater DataChannel inbound router (Req 12.3 – 12.6).
//!
//! Pure data-plane helpers invoked by the WebRTC layer whenever a
//! `DataChannelMessage` variant belongs to Theater mode. The actual
//! fan-out is kept here (rather than inside `webrtc::raw_frame`) so
//! every branch can be exercised by native unit tests without needing
//! a browser or live RTCPeerConnection.
//!
//! Six kinds of inbound messages are recognised:
//!
//! * [`TheaterInbound::Danmaku`] — a single danmaku entry. When the
//!   local user is the **owner** (star-topology hub) the danmaku is
//!   additionally enqueued onto the relay batcher so the 50 ms tick
//!   fans it out to every other viewer (Req 12.5 §28). Viewers only
//!   push onto the local overlay queue.
//! * [`TheaterInbound::DanmakuBatch`] — a merged batch forwarded by
//!   the owner. Viewers splat the entries onto the overlay queue; the
//!   batch never needs to be re-relayed because we already received
//!   it from the hub.
//! * [`TheaterInbound::SubtitleData`] / [`TheaterInbound::SubtitleClear`]
//!   — owner-authored subtitle state. Dispatched to the subtitle sync
//!   helpers on viewers.
//! * [`TheaterInbound::Playback`] — a playback progress broadcast
//!   used by viewers to keep their progress-bar HUD in sync with the
//!   owner.
//! * [`TheaterInbound::Chat`] — a theater-scoped chat bubble. Viewers
//!   append to the local chat log; owners append **and** requeue for
//!   relay to the remaining viewers (star topology per Req 12.6 §30).
//!
//! All messages are *ignored* when the active room id does not match
//! the inbound `room_id`. This protects the user from late-delivered
//! packets arriving after they left the theater (Req 12.2 §7).

use leptos::prelude::*;
use message::datachannel::{
  Danmaku, DanmakuBatch, DataChannelMessage, PlaybackProgress, SubtitleClear, SubtitleData,
  TheaterChatText,
};
use message::types::RoomId;

use super::chat_model::TheaterChatMessage;
use super::playback::apply_playback_progress;
use super::state::{TheaterRole, TheaterState};
use super::subtitle_sync::{apply_subtitle_clear, apply_subtitle_data};

/// Classified inbound theater message. Owning wrapper so callers can
/// pattern-match without also carrying the full [`DataChannelMessage`]
/// enum alongside.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TheaterInbound {
  /// A danmaku entry authored by a peer (or relayed by the owner).
  Danmaku(Danmaku),
  /// A merged batch of danmaku forwarded by the owner (Req 12.5 §28).
  DanmakuBatch(DanmakuBatch),
  /// A full subtitle track replacement from the owner.
  SubtitleData(SubtitleData),
  /// The owner cleared the subtitle track.
  SubtitleClear(SubtitleClear),
  /// Periodic playback progress broadcast from the owner (Req 12.4).
  Playback(PlaybackProgress),
  /// Theater-scoped chat bubble (Req 12.6 §30). Relayed through the
  /// owner so every viewer sees the same stream.
  Chat(TheaterChatText),
}

impl TheaterInbound {
  /// Room id the message is scoped to. Used by the router to filter
  /// out packets for stale theaters the user has already left.
  ///
  /// Single `Danmaku` frames do not carry a `room_id` on the wire
  /// (they travel over the peer's single DataChannel which is already
  /// room-scoped on connect), so the caller is expected to supply the
  /// active room id explicitly via `should_dispatch`.
  #[must_use]
  pub fn room_id(&self) -> Option<&RoomId> {
    match self {
      Self::Danmaku(_) => None,
      Self::DanmakuBatch(b) => Some(&b.room_id),
      Self::SubtitleData(data) => Some(&data.room_id),
      Self::SubtitleClear(clear) => Some(&clear.room_id),
      Self::Playback(p) => Some(&p.room_id),
      Self::Chat(c) => Some(&c.room_id),
    }
  }
}

/// Classify a raw [`DataChannelMessage`] into an optional theater
/// inbound event. Returns the original message boxed inside `Err` for
/// non-theater variants so the caller can fall through to its generic
/// handling path. Boxing keeps the `Err` payload cheap to return
/// regardless of how `DataChannelMessage` grows in the future.
pub fn classify(msg: DataChannelMessage) -> Result<TheaterInbound, Box<DataChannelMessage>> {
  match msg {
    DataChannelMessage::Danmaku(d) => Ok(TheaterInbound::Danmaku(d)),
    DataChannelMessage::DanmakuBatch(b) => Ok(TheaterInbound::DanmakuBatch(b)),
    DataChannelMessage::SubtitleData(d) => Ok(TheaterInbound::SubtitleData(d)),
    DataChannelMessage::SubtitleClear(c) => Ok(TheaterInbound::SubtitleClear(c)),
    DataChannelMessage::PlaybackProgress(p) => Ok(TheaterInbound::Playback(p)),
    DataChannelMessage::TheaterChatText(c) => Ok(TheaterInbound::Chat(c)),
    other => Err(Box::new(other)),
  }
}

/// Decide whether an inbound event should be dispatched to the local
/// theater state. Single `Danmaku` frames have no wire-level `room_id`
/// and are always accepted when the session is active; every other
/// variant must match the currently-active room.
#[must_use]
pub fn should_dispatch(active_room: Option<&RoomId>, inbound: &TheaterInbound) -> bool {
  let Some(active) = active_room else {
    return false;
  };
  match inbound.room_id() {
    Some(scope) => scope == active,
    None => true,
  }
}

/// Apply an inbound event to the local theater state.
///
/// Returns `true` when the event was accepted, `false` when it was
/// filtered out. The distinction is useful for tests and for telemetry.
pub fn apply(state: &TheaterState, inbound: TheaterInbound) -> bool {
  let active = state.room_id.get_untracked();
  if !should_dispatch(active.as_ref(), &inbound) {
    return false;
  }
  let is_owner = state.my_role.get_untracked() == TheaterRole::Owner;
  match inbound {
    TheaterInbound::Danmaku(d) => {
      // Every client — owner included — shows the danmaku locally.
      state.push_incoming_danmaku(d.clone());
      // Owner is the star-topology hub — additionally enqueue on the
      // relay batcher so the 50 ms tick fans this entry out to the
      // remaining viewers (Req 12.5 §28).
      if is_owner {
        state.with_danmaku_batcher::<()>(|b| {
          b.enqueue(d);
        });
      }
    }
    TheaterInbound::DanmakuBatch(batch) => {
      // Viewers receive merged batches from the owner; splat the
      // entries onto the overlay queue in arrival order. Owners
      // should not receive their own batch back, but guard anyway
      // so a misrouted frame cannot trigger an infinite relay loop.
      if is_owner {
        return false;
      }
      for entry in batch.entries {
        state.push_incoming_danmaku(entry);
      }
    }
    TheaterInbound::SubtitleData(data) => {
      apply_subtitle_data(state, data);
    }
    TheaterInbound::SubtitleClear(clear) => {
      apply_subtitle_clear(state, &clear);
    }
    TheaterInbound::Playback(p) => {
      return apply_playback_progress(state, &p);
    }
    TheaterInbound::Chat(c) => {
      apply_chat(state, c, is_owner);
    }
  }
  true
}

/// Push an inbound theater chat bubble onto the local log and, when
/// running as the owner, schedule a relay broadcast so every other
/// viewer receives the same payload.
///
/// The relay broadcast itself is delegated to the caller via the
/// `pending_chat_relay` signal to keep `dc_router` free of
/// `web_sys::*` side effects (so native unit tests remain viable).
fn apply_chat(state: &TheaterState, payload: TheaterChatText, is_owner: bool) {
  state.push_chat_message(TheaterChatMessage {
    id: state.next_chat_message_id(),
    sender_id: payload.sender_id.clone(),
    sender_name: payload.sender_id.to_string(),
    content: payload.content.clone(),
    sent_at_ms: payload.timestamp_nanos / 1_000_000,
    is_self: false,
  });
  if is_owner {
    // Hand off to the relay queue. The consumer (theater_page effect)
    // drains this queue, wraps each entry in a fresh DataChannel
    // envelope and broadcasts to the remaining viewers.
    state.enqueue_chat_relay(payload);
  }
}

#[cfg(test)]
mod tests;

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests;
