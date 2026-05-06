//! Owner-side RAF-based CPU drop-rate monitor (Req 12.2 §7a).
//!
//! The theater requirement mandates that the owner side automatically
//! reduce danmaku rendering density and video stream quality whenever
//! the local frame drop rate exceeds 30 % for more than 10 seconds.
//!
//! We infer the drop rate by counting how many `requestAnimationFrame`
//! callbacks arrived within a sliding one-second window versus the
//! browser's nominal 60 Hz refresh rate. Because the raw signal is
//! noisy (GC pauses, tab background throttling), we accumulate a hold
//! timer only when the threshold is breached — mirroring the pattern
//! already used by the `bufferedAmount`-driven [`resource_monitor`].
//!
//! The helpers are pure Rust so the boundary conditions (first sample,
//! ambiguous idle window, recovery after reload) are unit-testable
//! under native `cargo test`.

/// Nominal browser refresh rate in frames per second. Used to derive
/// the "expected" RAF count inside the one-second sampling window.
pub const NOMINAL_FPS: u32 = 60;

/// Drop-rate percentage above which the hold timer starts running.
/// Req 12.2 §7a: "frame drop rate exceeds 30 %".
pub const DROP_RATE_THRESHOLD_PERCENT: u32 = 30;

/// Seconds the drop-rate must stay above
/// [`DROP_RATE_THRESHOLD_PERCENT`] before triggering a degradation
/// action.
pub const DEGRADATION_HOLD_SECONDS: u32 = 10;

/// Snapshot of the frame-drop monitor's internal counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FrameDropSnapshot {
  /// Consecutive seconds the frame drop rate has been above the
  /// threshold.
  pub elevated_seconds: u32,
  /// RAF callback count observed in the current sampling window.
  pub frames_in_window: u32,
}

/// Action the monitor wants the caller to take once the current
/// sampling window closes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameDropAction {
  /// No action required.
  Hold,
  /// Drop-rate has been elevated for long enough — reduce danmaku
  /// density and step the video quality tier down.
  Degrade,
}

/// Compute the observed drop-rate percentage for a single one-second
/// sampling window. Returns a value clamped to `[0, 100]`.
#[must_use]
pub fn drop_rate_percent(frames_observed: u32, nominal_fps: u32) -> u32 {
  if nominal_fps == 0 {
    return 0;
  }
  let expected = nominal_fps;
  let missed = expected.saturating_sub(frames_observed);
  let ratio = (missed.saturating_mul(100)) / expected;
  ratio.min(100)
}

/// Apply one second of RAF samples to the snapshot and decide whether
/// the owner should degrade now. The caller resets the window-level
/// counter (`frames_in_window`) before calling back in the next
/// second.
///
/// The function is pure: counters and the returned action depend only
/// on the supplied arguments.
#[must_use]
pub fn evaluate_second(
  snapshot: &mut FrameDropSnapshot,
  frames_observed: u32,
  nominal_fps: u32,
) -> FrameDropAction {
  snapshot.frames_in_window = frames_observed;
  let drop_rate = drop_rate_percent(frames_observed, nominal_fps);
  if drop_rate > DROP_RATE_THRESHOLD_PERCENT {
    snapshot.elevated_seconds = snapshot.elevated_seconds.saturating_add(1);
    if snapshot.elevated_seconds >= DEGRADATION_HOLD_SECONDS {
      snapshot.elevated_seconds = 0;
      return FrameDropAction::Degrade;
    }
  } else {
    // Recovered — reset the hold counter so future elevations must
    // start from zero again (mirrors the `resource_monitor`'s
    // grey-zone handling).
    snapshot.elevated_seconds = 0;
  }
  FrameDropAction::Hold
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn drop_rate_under_budget_returns_zero() {
    assert_eq!(drop_rate_percent(60, 60), 0);
    assert_eq!(drop_rate_percent(70, 60), 0);
  }

  #[test]
  fn drop_rate_returns_percentage() {
    assert_eq!(drop_rate_percent(30, 60), 50);
    assert_eq!(drop_rate_percent(42, 60), 30);
    assert_eq!(drop_rate_percent(0, 60), 100);
  }

  #[test]
  fn drop_rate_handles_zero_nominal() {
    assert_eq!(drop_rate_percent(42, 0), 0);
  }

  #[test]
  fn evaluate_degrades_after_hold_period() {
    let mut snap = FrameDropSnapshot::default();
    // 40 frames / 60 nominal = 33 % drop rate — above the 30 % gate.
    for _ in 0..(DEGRADATION_HOLD_SECONDS - 1) {
      assert_eq!(
        evaluate_second(&mut snap, 40, NOMINAL_FPS),
        FrameDropAction::Hold
      );
    }
    assert_eq!(
      evaluate_second(&mut snap, 40, NOMINAL_FPS),
      FrameDropAction::Degrade
    );
    assert_eq!(snap.elevated_seconds, 0);
  }

  #[test]
  fn evaluate_resets_counter_when_recovering() {
    let mut snap = FrameDropSnapshot::default();
    assert_eq!(
      evaluate_second(&mut snap, 40, NOMINAL_FPS),
      FrameDropAction::Hold
    );
    assert_eq!(snap.elevated_seconds, 1);

    // Dropped back under the threshold — reset.
    assert_eq!(
      evaluate_second(&mut snap, 55, NOMINAL_FPS),
      FrameDropAction::Hold
    );
    assert_eq!(snap.elevated_seconds, 0);
  }

  #[test]
  fn evaluate_requires_strict_above_threshold() {
    let mut snap = FrameDropSnapshot::default();
    // Exactly at the 30 % threshold — should hold and keep the
    // counter at zero (boundary condition).
    assert_eq!(
      evaluate_second(&mut snap, 42, NOMINAL_FPS),
      FrameDropAction::Hold
    );
    assert_eq!(snap.elevated_seconds, 0);
  }
}
