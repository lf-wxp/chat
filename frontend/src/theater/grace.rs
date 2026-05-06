//! Owner disconnect grace window (Req 12.2 §6a).
//!
//! Pure-function helpers that drive the 30-second grace window used
//! when the theater owner unexpectedly disconnects. The UI layer
//! starts a ticker (`set_interval` every 1 s) and calls these helpers
//! to derive the remaining seconds + whether the deadline has been
//! reached.
//!
//! Separating this out of the component file lets us unit-test the
//! boundary conditions (exact-30s, past-30s, clock skew) without
//! needing a browser.

/// Total grace window length in seconds (Req 12.2 §6a).
pub const GRACE_WINDOW_SECONDS: u32 = 30;

/// Compute how many seconds are still left in the grace window,
/// clamped at 0. The clamp guarantees the UI never shows a negative
/// countdown even if the ticker fires past the deadline.
#[must_use]
pub fn compute_grace_remaining(started_at_ms: u64, now_ms: u64, total_s: u32) -> u32 {
  let elapsed_ms = now_ms.saturating_sub(started_at_ms);
  let elapsed_s = u32::try_from(elapsed_ms / 1_000).unwrap_or(u32::MAX);
  total_s.saturating_sub(elapsed_s)
}

/// Whether the grace window has fully elapsed and the UI should
/// switch from "reconnecting" to "offline" messaging.
#[must_use]
pub fn is_grace_expired(started_at_ms: u64, now_ms: u64, total_s: u32) -> bool {
  compute_grace_remaining(started_at_ms, now_ms, total_s) == 0
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn remaining_starts_at_full_window() {
    assert_eq!(compute_grace_remaining(1_000, 1_000, 30), 30);
  }

  #[test]
  fn remaining_ticks_down_each_second() {
    assert_eq!(compute_grace_remaining(0, 5_000, 30), 25);
    // 15_500 ms / 1_000 = 15 seconds elapsed (integer division), so
    // remaining is 30 - 15 = 15 rather than 14.
    assert_eq!(compute_grace_remaining(0, 15_500, 30), 15);
    assert_eq!(compute_grace_remaining(0, 16_000, 30), 14);
  }

  #[test]
  fn remaining_clamps_at_zero_after_deadline() {
    assert_eq!(compute_grace_remaining(0, 40_000, 30), 0);
    assert_eq!(compute_grace_remaining(0, u64::MAX, 30), 0);
  }

  #[test]
  fn remaining_handles_clock_rollback_gracefully() {
    // `now_ms < started_ms` — treat as "just started" rather than
    // underflow.
    assert_eq!(compute_grace_remaining(10_000, 5_000, 30), 30);
  }

  #[test]
  fn is_grace_expired_matches_boundary() {
    assert!(!is_grace_expired(0, 29_999, 30));
    assert!(is_grace_expired(0, 30_000, 30));
    assert!(is_grace_expired(0, 99_999, 30));
  }
}
