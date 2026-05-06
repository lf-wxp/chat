//! Theater playback helpers (Req 12.3 / 12.4).
//!
//! Pure logic used by the video player / playback controls:
//!
//! * [`should_broadcast_progress`] — rate-limits the owner's
//!   `PlaybackProgress` broadcast to at most one frame per
//!   [`PROGRESS_BROADCAST_INTERVAL_MS`] so we don't saturate the
//!   DataChannel.
//! * [`needs_seek`] — decides whether a viewer should re-seek the
//!   local `<video>` element to match the owner's timestamp.
//!   Prevents micro-jitter by ignoring deltas under
//!   [`SEEK_TOLERANCE_MS`].
//! * [`format_timestamp`] — human-readable `mm:ss` / `hh:mm:ss`
//!   formatting for the seek bar label.
//!
//! None of these helpers touch `web_sys` so they can be unit-tested
//! without a browser environment.

use leptos::prelude::*;
use message::datachannel::PlaybackProgress;
use message::types::RoomId;

use super::state::{PlaybackSnapshot, TheaterState};

/// Minimum interval (milliseconds) between two outbound
/// `PlaybackProgress` frames — once every 500 ms keeps viewers in
/// sync within a single seek tick while staying well under the
/// DataChannel throughput budget.
pub const PROGRESS_BROADCAST_INTERVAL_MS: u64 = 500;

/// A viewer re-seeks its `<video>` element only when the local
/// position drifts by more than this many milliseconds from the
/// owner's timestamp. Smaller values cause constant micro-seeks.
pub const SEEK_TOLERANCE_MS: i64 = 1_500;

/// Whether the owner should send a new `PlaybackProgress` frame.
///
/// Broadcast gating covers four cases:
/// 1. First frame (no previous timestamp) → always send.
/// 2. Pause-state change → always send so viewers flip immediately.
/// 3. Regular tick → only when `now - last >= interval`.
/// 4. User-initiated seek (large forward/backward jump) → always send.
#[must_use]
pub fn should_broadcast_progress(
  last_sent_ms: Option<u64>,
  now_ms: u64,
  last_snapshot: PlaybackSnapshot,
  next_snapshot: PlaybackSnapshot,
) -> bool {
  let Some(last) = last_sent_ms else {
    return true;
  };
  if last_snapshot.is_paused != next_snapshot.is_paused {
    return true;
  }
  let dt = now_ms.saturating_sub(last);
  if dt >= PROGRESS_BROADCAST_INTERVAL_MS {
    return true;
  }
  // Large position jump between two ticks indicates a user seek.
  let jump = next_snapshot
    .current_time_ms
    .abs_diff(last_snapshot.current_time_ms);
  jump >= 2_000
}

/// Decide whether a viewer should re-seek to match the owner's
/// timestamp (`remote_time_ms`). Returns `None` when the drift is
/// inside [`SEEK_TOLERANCE_MS`] — the caller should leave the
/// `<video>` element alone.
#[must_use]
pub fn needs_seek(local_time_ms: u64, remote_time_ms: u64) -> Option<u64> {
  let drift = (remote_time_ms as i64) - (local_time_ms as i64);
  if drift.abs() > SEEK_TOLERANCE_MS {
    Some(remote_time_ms)
  } else {
    None
  }
}

/// Format a millisecond timestamp as `mm:ss` or `hh:mm:ss`.
#[must_use]
pub fn format_timestamp(ms: u64) -> String {
  let total_seconds = ms / 1_000;
  let hours = total_seconds / 3_600;
  let minutes = (total_seconds % 3_600) / 60;
  let seconds = total_seconds % 60;
  if hours > 0 {
    format!("{hours}:{minutes:02}:{seconds:02}")
  } else {
    format!("{minutes}:{seconds:02}")
  }
}

/// Build a `PlaybackProgress` frame from the local snapshot.
#[must_use]
pub fn build_progress_frame(
  room_id: RoomId,
  snapshot: PlaybackSnapshot,
  timestamp_nanos: u64,
) -> PlaybackProgress {
  PlaybackProgress {
    room_id,
    current_time_ms: snapshot.current_time_ms,
    duration_ms: snapshot.duration_ms,
    is_paused: snapshot.is_paused,
    timestamp_nanos,
  }
}

/// Apply an incoming [`PlaybackProgress`] frame to the reactive state.
///
/// Viewers call this from the DataChannel router so the overlay,
/// controls and video-element effect pick up the new target
/// immediately. Returns `true` when the snapshot actually changed.
pub fn apply_playback_progress(state: &TheaterState, msg: &PlaybackProgress) -> bool {
  // Ignore frames routed from a different room (defensive — the
  // router should have filtered these out already).
  if !state
    .room_id
    .with_untracked(|r| r.as_ref().is_some_and(|id| id == &msg.room_id))
  {
    return false;
  }
  let next = PlaybackSnapshot {
    current_time_ms: msg.current_time_ms,
    duration_ms: msg.duration_ms,
    is_paused: msg.is_paused,
  };
  let changed = state.playback.with_untracked(|prev| *prev != next);
  if changed {
    state.playback.set(next);
  }
  changed
}

#[cfg(test)]
mod tests {
  use super::*;

  fn snap(time_ms: u64, paused: bool) -> PlaybackSnapshot {
    PlaybackSnapshot {
      current_time_ms: time_ms,
      duration_ms: 600_000,
      is_paused: paused,
    }
  }

  #[test]
  fn broadcast_first_frame_always_sends() {
    assert!(should_broadcast_progress(
      None,
      0,
      snap(0, false),
      snap(0, false)
    ));
  }

  #[test]
  fn broadcast_respects_rate_limit() {
    let last = snap(0, false);
    let next = snap(100, false);
    assert!(!should_broadcast_progress(Some(1_000), 1_100, last, next));
    assert!(should_broadcast_progress(Some(1_000), 1_500, last, next));
  }

  #[test]
  fn broadcast_forces_on_pause_flip() {
    let last = snap(5_000, false);
    let next = snap(5_050, true);
    // Only 50 ms elapsed but pause flipped → must broadcast.
    assert!(should_broadcast_progress(Some(10_000), 10_050, last, next));
  }

  #[test]
  fn broadcast_forces_on_seek_jump() {
    let last = snap(5_000, false);
    let next = snap(60_000, false);
    // Under-rate but 55 s jump → must broadcast.
    assert!(should_broadcast_progress(Some(10_000), 10_100, last, next));
  }

  #[test]
  fn no_seek_inside_tolerance() {
    assert_eq!(needs_seek(10_000, 10_800), None);
    assert_eq!(needs_seek(10_000, 9_500), None);
  }

  #[test]
  fn seeks_when_drift_exceeds_tolerance() {
    assert_eq!(needs_seek(10_000, 12_000), Some(12_000));
    assert_eq!(needs_seek(20_000, 10_000), Some(10_000));
  }

  #[test]
  fn timestamp_format_under_hour() {
    assert_eq!(format_timestamp(0), "0:00");
    assert_eq!(format_timestamp(5_000), "0:05");
    assert_eq!(format_timestamp(65_000), "1:05");
    assert_eq!(format_timestamp(125_000), "2:05");
  }

  #[test]
  fn timestamp_format_with_hours() {
    assert_eq!(format_timestamp(3_600_000), "1:00:00");
    assert_eq!(format_timestamp(3_725_000), "1:02:05");
  }
}
