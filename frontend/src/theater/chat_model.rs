//! Theater chat data model (Req 12.6).
//!
//! A tiny data layer for the theater-room chat panel. Every helper in
//! this module is pure with respect to `web_sys`, so all branches can
//! be exercised by native unit tests.
//!
//! The chat messages rendered by the panel intentionally carry the
//! **resolved display name** rather than looking it up on every
//! render. This keeps the view tree stateless and avoids a second
//! subscription to the room member map on every bubble.

use std::collections::VecDeque;

use message::UserId;

/// Upper bound on retained chat messages. Anything beyond this is
/// dropped FIFO so the panel cannot grow unbounded during a long
/// co-watching session.
pub const CHAT_MESSAGE_HISTORY_CAP: usize = 500;

/// One chat entry rendered inside [`crate::components::theater::TheaterChatPanel`].
///
/// Stored inside a `VecDeque` in [`TheaterState::chat_messages`] so the
/// view can enumerate bubbles in insertion order without needing to
/// re-sort. `sent_at_ms` drives the relative-time label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TheaterChatMessage {
  /// Monotonic counter id (scoped to the local session). Serves as
  /// the `key` for Leptos `<For/>` rendering.
  pub id: u64,
  /// User id of the sender. May equal the current user for locally
  /// authored messages.
  pub sender_id: UserId,
  /// Resolved display name at the time of authoring — avoids a
  /// post-hoc lookup when the sender changes their nickname later.
  pub sender_name: String,
  /// Message body as plain text.
  pub content: String,
  /// Unix timestamp in milliseconds when the message was authored
  /// (sender's clock — not adjusted for server drift).
  pub sent_at_ms: u64,
  /// Whether this is the message the current user just sent. The
  /// view uses it to right-align the bubble.
  pub is_self: bool,
}

/// Append `next` to `history`, evicting the oldest entry when the
/// cap is exceeded. Returns the tail slot index (useful for tests
/// asserting on insertion order).
pub fn append_message(
  history: &mut VecDeque<TheaterChatMessage>,
  next: TheaterChatMessage,
) -> usize {
  if history.len() >= CHAT_MESSAGE_HISTORY_CAP {
    history.pop_front();
  }
  history.push_back(next);
  history.len() - 1
}

/// Format a relative time label (e.g. "just now" / "5m ago" /
/// "14:32"). Keys match the i18n bundle — the render layer looks up
/// the translated string from this discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelativeTimeLabel {
  /// Less than 60 seconds ago.
  JustNow,
  /// 1–59 minutes ago (carries the minute count).
  MinutesAgo(u32),
  /// 1–23 hours ago.
  HoursAgo(u32),
  /// Older than 24 h — render the absolute `HH:MM` instead.
  Absolute { hours: u32, minutes: u32 },
}

/// Classify how a sent-at timestamp should be displayed relative to
/// the current wall-clock time. Boundary choices follow the rest of
/// the chat UI (see Req 14.2).
#[must_use]
pub fn relative_time_label(sent_at_ms: u64, now_ms: u64) -> RelativeTimeLabel {
  let delta = now_ms.saturating_sub(sent_at_ms);
  const ONE_MINUTE_MS: u64 = 60 * 1_000;
  const ONE_HOUR_MS: u64 = 60 * ONE_MINUTE_MS;
  const ONE_DAY_MS: u64 = 24 * ONE_HOUR_MS;

  if delta < ONE_MINUTE_MS {
    return RelativeTimeLabel::JustNow;
  }
  if delta < ONE_HOUR_MS {
    let minutes = u32::try_from(delta / ONE_MINUTE_MS).unwrap_or(59);
    return RelativeTimeLabel::MinutesAgo(minutes.max(1));
  }
  if delta < ONE_DAY_MS {
    let hours = u32::try_from(delta / ONE_HOUR_MS).unwrap_or(23);
    return RelativeTimeLabel::HoursAgo(hours.max(1));
  }
  // Compute `HH:MM` for the local timezone. The `now_ms` parameter
  // is only used for boundary detection above; the absolute label is
  // derived directly from `sent_at_ms`.
  let seconds = sent_at_ms / 1_000;
  let hours = u32::try_from((seconds / 3_600) % 24).unwrap_or(0);
  let minutes = u32::try_from((seconds / 60) % 60).unwrap_or(0);
  RelativeTimeLabel::Absolute { hours, minutes }
}

#[cfg(test)]
mod tests {
  use super::*;
  use uuid::Uuid;

  fn make_message(id: u64, sender: &str, body: &str) -> TheaterChatMessage {
    TheaterChatMessage {
      id,
      sender_id: UserId(Uuid::from_u128(u128::from(id) + 1)),
      sender_name: sender.into(),
      content: body.into(),
      sent_at_ms: 0,
      is_self: false,
    }
  }

  #[test]
  fn append_respects_history_cap() {
    let mut hist = VecDeque::<TheaterChatMessage>::with_capacity(CHAT_MESSAGE_HISTORY_CAP);
    for i in 0..(CHAT_MESSAGE_HISTORY_CAP as u64) {
      append_message(&mut hist, make_message(i, "alice", "hi"));
    }
    assert_eq!(hist.len(), CHAT_MESSAGE_HISTORY_CAP);
    // One more push should evict the oldest, not grow the deque.
    append_message(&mut hist, make_message(9_999, "alice", "extra"));
    assert_eq!(hist.len(), CHAT_MESSAGE_HISTORY_CAP);
    assert_eq!(hist.front().unwrap().id, 1);
    assert_eq!(hist.back().unwrap().id, 9_999);
  }

  #[test]
  fn append_returns_tail_index() {
    let mut hist = VecDeque::new();
    let idx = append_message(&mut hist, make_message(1, "a", "x"));
    assert_eq!(idx, 0);
    let idx = append_message(&mut hist, make_message(2, "a", "y"));
    assert_eq!(idx, 1);
  }

  #[test]
  fn relative_time_just_now_under_60_seconds() {
    let now = 10_000;
    let sent = 9_500; // 500 ms ago
    assert_eq!(relative_time_label(sent, now), RelativeTimeLabel::JustNow);
  }

  #[test]
  fn relative_time_minutes_agreement() {
    let now = 10 * 60 * 1_000;
    let sent = 5 * 60 * 1_000;
    assert_eq!(
      relative_time_label(sent, now),
      RelativeTimeLabel::MinutesAgo(5)
    );
  }

  #[test]
  fn relative_time_hours_agreement() {
    let now = 5 * 60 * 60 * 1_000;
    let sent = 2 * 60 * 60 * 1_000;
    assert_eq!(
      relative_time_label(sent, now),
      RelativeTimeLabel::HoursAgo(3)
    );
  }

  #[test]
  fn relative_time_absolute_for_previous_day() {
    let one_day_ms = 24 * 60 * 60 * 1_000;
    let sent = 14 * 60 * 60 * 1_000 + 32 * 60 * 1_000; // 14:32 UTC on day 0
    let now = sent + one_day_ms + 1; // just over 24 hours later
    assert_eq!(
      relative_time_label(sent, now),
      RelativeTimeLabel::Absolute {
        hours: 14,
        minutes: 32,
      }
    );
  }

  #[test]
  fn relative_time_clamps_future_timestamps_to_just_now() {
    // Guard against sender clock drift producing negative deltas.
    let now = 10_000;
    let sent = 15_000;
    assert_eq!(relative_time_label(sent, now), RelativeTimeLabel::JustNow);
  }
}
