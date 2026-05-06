//! Owner-side resource monitor (Req 12.2 §4a).
//!
//! Pure-function helpers that drive the auto-degradation / auto-restore
//! logic for the theater owner's video stream quality. The UI layer
//! runs a 1-second polling interval and calls these helpers to derive
//! the next quality tier based on the aggregate `bufferedAmount` across
//! all viewer DataChannels.
//!
//! Thresholds from the requirement:
//!
//! * **Degradation**: `bufferedAmount` consistently > 1 MB for > 5 s
//! * **Recovery**: `bufferedAmount` < 512 KB for > 10 s
//! * **High-load warning**: estimated outbound bandwidth utilization > 80%
//!
//! Separating this from the component keeps the logic native-testable.

use super::state::QualityTier;

/// Threshold above which the owner is considered "high load" (bytes).
/// Req 12.2 §4a: "bufferedAmount consistently exceeds 1MB".
pub const DEGRADATION_THRESHOLD_BYTES: u32 = 1_024 * 1_024;

/// Threshold below which recovery can begin (bytes).
/// Req 12.2 §4a: "bufferedAmount drops below 512KB".
pub const RECOVERY_THRESHOLD_BYTES: u32 = 512 * 1_024;

/// Seconds the buffer must stay above the degradation threshold
/// before quality is reduced.
pub const DEGRADATION_HOLD_SECONDS: u32 = 5;

/// Seconds the buffer must stay below the recovery threshold
/// before quality is restored one step.
pub const RECOVERY_HOLD_SECONDS: u32 = 10;

/// Snapshot of the resource monitor's internal counters. Stored in a
/// signal so the polling interval can accumulate state across ticks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MonitorSnapshot {
  /// Consecutive seconds the aggregate bufferedAmount has been above
  /// [`DEGRADATION_THRESHOLD_BYTES`].
  pub above_threshold_seconds: u32,
  /// Consecutive seconds the aggregate bufferedAmount has been below
  /// [`RECOVERY_THRESHOLD_BYTES`].
  pub below_threshold_seconds: u32,
}

/// Result of a single monitor tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorAction {
  /// No quality change needed.
  Hold,
  /// Degrade one step (e.g. 1080p → 720p, or 720p → 480p).
  Degrade,
  /// Restore one step (e.g. 480p → 720p, or 720p → 1080p).
  Restore,
}

/// Compute the next monitor action given the current aggregate
/// `bufferedAmount` (sum across all viewer DataChannels) and the
/// running snapshot counters.
///
/// This function is pure — it updates `snapshot` in place and returns
/// the action the caller should take. The caller is responsible for
/// applying the quality tier change and resetting the snapshot when
/// a tier transition occurs.
#[must_use]
pub fn evaluate_tick(
  snapshot: &mut MonitorSnapshot,
  aggregate_buffered: u32,
  current_tier: QualityTier,
) -> MonitorAction {
  if aggregate_buffered > DEGRADATION_THRESHOLD_BYTES {
    snapshot.above_threshold_seconds = snapshot.above_threshold_seconds.saturating_add(1);
    snapshot.below_threshold_seconds = 0;

    if snapshot.above_threshold_seconds >= DEGRADATION_HOLD_SECONDS
      && current_tier != QualityTier::Low
    {
      // Reset the counter so the next degradation step requires
      // another full hold period.
      snapshot.above_threshold_seconds = 0;
      return MonitorAction::Degrade;
    }
  } else if aggregate_buffered < RECOVERY_THRESHOLD_BYTES {
    snapshot.below_threshold_seconds = snapshot.below_threshold_seconds.saturating_add(1);
    snapshot.above_threshold_seconds = 0;

    if snapshot.below_threshold_seconds >= RECOVERY_HOLD_SECONDS
      && current_tier != QualityTier::HighDefinition
    {
      snapshot.below_threshold_seconds = 0;
      return MonitorAction::Restore;
    }
  } else {
    // In the "grey zone" between thresholds — reset both counters.
    snapshot.above_threshold_seconds = 0;
    snapshot.below_threshold_seconds = 0;
  }

  MonitorAction::Hold
}

/// Whether the owner should display the "high load" warning banner.
/// Triggered when the aggregate bufferedAmount exceeds 80% of the
/// degradation threshold (i.e. approaching overload).
#[must_use]
pub fn is_high_load(aggregate_buffered: u32) -> bool {
  aggregate_buffered > (DEGRADATION_THRESHOLD_BYTES * 4 / 5)
}

/// Step the quality tier down by one notch.
#[must_use]
pub fn degrade_tier(current: QualityTier) -> QualityTier {
  match current {
    QualityTier::HighDefinition => QualityTier::StandardDefinition,
    QualityTier::StandardDefinition => QualityTier::Low,
    QualityTier::Low => QualityTier::Low,
  }
}

/// Step the quality tier up by one notch.
/// Req 12.2 §4a: "first restore frame rate, then after 10s restore
/// resolution" — modelled as two discrete restore steps.
#[must_use]
pub fn restore_tier(current: QualityTier) -> QualityTier {
  match current {
    QualityTier::Low => QualityTier::StandardDefinition,
    QualityTier::StandardDefinition => QualityTier::HighDefinition,
    QualityTier::HighDefinition => QualityTier::HighDefinition,
  }
}

// ── Bandwidth estimation via getStats() (Req 12.2 §4a) ─────────────

/// Default capacity estimate for 1080p @ 30 fps (bits per second).
/// Used as the denominator when computing utilization percentage.
pub const DEFAULT_CAPACITY_BPS: u64 = 5_000_000;

/// Utilization percentage above which the owner is considered
/// bandwidth-saturated. Req 12.2 §4a: "80% utilization".
pub const BANDWIDTH_HIGH_UTILIZATION_PERCENT: u8 = 80;

/// Snapshot of the bandwidth estimator's state across ticks.
/// The estimator derives throughput from `Δ bytesSent / Δ time`
/// reported by the `outbound-rtp` stats.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BandwidthSnapshot {
  /// Total `bytesSent` across all outbound-rtp reports at the
  /// previous tick (sum of all peers).
  pub prev_bytes_sent: u64,
  /// Timestamp (milliseconds) of the previous measurement.
  pub prev_timestamp_ms: u64,
  /// Most recently computed throughput in bits per second.
  pub current_throughput_bps: u64,
}

/// Result of a bandwidth estimation tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BandwidthEstimate {
  /// Estimated outbound throughput in bits per second.
  pub throughput_bps: u64,
  /// Utilization percentage relative to `capacity_bps`.
  pub utilization_percent: u8,
  /// Whether the utilization exceeds the high-load threshold.
  pub is_saturated: bool,
}

/// Evaluate a bandwidth estimation tick given the aggregate
/// `bytesSent` from all outbound-rtp stats and the current
/// timestamp. Returns the computed estimate and updates the
/// snapshot in place.
///
/// The first tick (when `prev_timestamp_ms == 0`) produces a zero
/// estimate because there is no delta to compute from.
#[must_use]
pub fn evaluate_bandwidth(
  snapshot: &mut BandwidthSnapshot,
  total_bytes_sent: u64,
  now_ms: u64,
  capacity_bps: u64,
) -> BandwidthEstimate {
  let delta_ms = now_ms.saturating_sub(snapshot.prev_timestamp_ms);
  let delta_bytes = total_bytes_sent.saturating_sub(snapshot.prev_bytes_sent);
  let is_first_tick = snapshot.prev_timestamp_ms == 0;

  // Update snapshot for the next tick.
  snapshot.prev_bytes_sent = total_bytes_sent;
  snapshot.prev_timestamp_ms = now_ms;

  // Guard: first tick or zero elapsed time — cannot compute rate.
  if is_first_tick || delta_ms == 0 {
    return BandwidthEstimate {
      throughput_bps: snapshot.current_throughput_bps,
      utilization_percent: 0,
      is_saturated: false,
    };
  }

  // bits per second = (delta_bytes * 8 * 1000) / delta_ms
  let throughput_bps = delta_bytes
    .saturating_mul(8)
    .saturating_mul(1_000)
    .checked_div(delta_ms)
    .unwrap_or(0);

  snapshot.current_throughput_bps = throughput_bps;

  let utilization_percent = throughput_bps
    .saturating_mul(100)
    .checked_div(capacity_bps)
    .map_or(100, |v| u8::try_from(v).unwrap_or(u8::MAX));

  let is_saturated = utilization_percent >= BANDWIDTH_HIGH_UTILIZATION_PERCENT;

  BandwidthEstimate {
    throughput_bps,
    utilization_percent,
    is_saturated,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn degrade_steps_down_correctly() {
    assert_eq!(
      degrade_tier(QualityTier::HighDefinition),
      QualityTier::StandardDefinition
    );
    assert_eq!(
      degrade_tier(QualityTier::StandardDefinition),
      QualityTier::Low
    );
    assert_eq!(degrade_tier(QualityTier::Low), QualityTier::Low);
  }

  #[test]
  fn restore_steps_up_correctly() {
    assert_eq!(
      restore_tier(QualityTier::Low),
      QualityTier::StandardDefinition
    );
    assert_eq!(
      restore_tier(QualityTier::StandardDefinition),
      QualityTier::HighDefinition
    );
    assert_eq!(
      restore_tier(QualityTier::HighDefinition),
      QualityTier::HighDefinition
    );
  }

  #[test]
  fn evaluate_degrades_after_hold_period() {
    let mut snap = MonitorSnapshot::default();
    let high_buffer = DEGRADATION_THRESHOLD_BYTES + 1;

    // First 4 seconds: hold
    for _ in 0..4 {
      assert_eq!(
        evaluate_tick(&mut snap, high_buffer, QualityTier::HighDefinition),
        MonitorAction::Hold
      );
    }
    // 5th second: degrade
    assert_eq!(
      evaluate_tick(&mut snap, high_buffer, QualityTier::HighDefinition),
      MonitorAction::Degrade
    );
    // Counter resets after degradation
    assert_eq!(snap.above_threshold_seconds, 0);
  }

  #[test]
  fn evaluate_restores_after_recovery_period() {
    let mut snap = MonitorSnapshot::default();
    let low_buffer = RECOVERY_THRESHOLD_BYTES - 1;

    // First 9 seconds: hold
    for _ in 0..9 {
      assert_eq!(
        evaluate_tick(&mut snap, low_buffer, QualityTier::Low),
        MonitorAction::Hold
      );
    }
    // 10th second: restore
    assert_eq!(
      evaluate_tick(&mut snap, low_buffer, QualityTier::Low),
      MonitorAction::Restore
    );
    assert_eq!(snap.below_threshold_seconds, 0);
  }

  #[test]
  fn evaluate_does_not_degrade_below_low() {
    let mut snap = MonitorSnapshot {
      above_threshold_seconds: 10,
      below_threshold_seconds: 0,
    };
    let high_buffer = DEGRADATION_THRESHOLD_BYTES + 1;
    // Already at Low — should hold even with high buffer.
    assert_eq!(
      evaluate_tick(&mut snap, high_buffer, QualityTier::Low),
      MonitorAction::Hold
    );
  }

  #[test]
  fn evaluate_does_not_restore_above_hd() {
    let mut snap = MonitorSnapshot {
      above_threshold_seconds: 0,
      below_threshold_seconds: 20,
    };
    let low_buffer = 0;
    assert_eq!(
      evaluate_tick(&mut snap, low_buffer, QualityTier::HighDefinition),
      MonitorAction::Hold
    );
  }

  #[test]
  fn grey_zone_resets_both_counters() {
    let mut snap = MonitorSnapshot {
      above_threshold_seconds: 3,
      below_threshold_seconds: 5,
    };
    let mid_buffer = RECOVERY_THRESHOLD_BYTES + 100;
    assert_eq!(
      evaluate_tick(&mut snap, mid_buffer, QualityTier::StandardDefinition),
      MonitorAction::Hold
    );
    assert_eq!(snap.above_threshold_seconds, 0);
    assert_eq!(snap.below_threshold_seconds, 0);
  }

  #[test]
  fn is_high_load_threshold() {
    assert!(!is_high_load(0));
    assert!(!is_high_load(DEGRADATION_THRESHOLD_BYTES * 4 / 5));
    assert!(is_high_load(DEGRADATION_THRESHOLD_BYTES * 4 / 5 + 1));
    assert!(is_high_load(DEGRADATION_THRESHOLD_BYTES));
  }

  // ── Bandwidth estimation tests ──────────────────────────────────────

  #[test]
  fn bandwidth_first_tick_returns_zero() {
    let mut snap = BandwidthSnapshot::default();
    let est = evaluate_bandwidth(&mut snap, 100_000, 1_000, DEFAULT_CAPACITY_BPS);
    // First tick has no previous reference — throughput is zero.
    assert_eq!(est.throughput_bps, 0);
    assert_eq!(est.utilization_percent, 0);
    assert!(!est.is_saturated);
    // Snapshot is updated for the next tick.
    assert_eq!(snap.prev_bytes_sent, 100_000);
    assert_eq!(snap.prev_timestamp_ms, 1_000);
  }

  #[test]
  fn bandwidth_computes_throughput_correctly() {
    let mut snap = BandwidthSnapshot {
      prev_bytes_sent: 0,
      prev_timestamp_ms: 1_000,
      current_throughput_bps: 0,
    };
    // 625_000 bytes in 1 second = 5_000_000 bps (5 Mbps).
    let est = evaluate_bandwidth(&mut snap, 625_000, 2_000, DEFAULT_CAPACITY_BPS);
    assert_eq!(est.throughput_bps, 5_000_000);
    assert_eq!(est.utilization_percent, 100);
    assert!(est.is_saturated);
  }

  #[test]
  fn bandwidth_below_threshold_not_saturated() {
    let mut snap = BandwidthSnapshot {
      prev_bytes_sent: 0,
      prev_timestamp_ms: 1_000,
      current_throughput_bps: 0,
    };
    // 250_000 bytes in 1 second = 2_000_000 bps (2 Mbps) = 40% of 5 Mbps.
    let est = evaluate_bandwidth(&mut snap, 250_000, 2_000, DEFAULT_CAPACITY_BPS);
    assert_eq!(est.throughput_bps, 2_000_000);
    assert_eq!(est.utilization_percent, 40);
    assert!(!est.is_saturated);
  }

  #[test]
  fn bandwidth_at_80_percent_is_saturated() {
    let mut snap = BandwidthSnapshot {
      prev_bytes_sent: 0,
      prev_timestamp_ms: 1_000,
      current_throughput_bps: 0,
    };
    // 500_000 bytes in 1 second = 4_000_000 bps = 80% of 5 Mbps.
    let est = evaluate_bandwidth(&mut snap, 500_000, 2_000, DEFAULT_CAPACITY_BPS);
    assert_eq!(est.throughput_bps, 4_000_000);
    assert_eq!(est.utilization_percent, 80);
    assert!(est.is_saturated);
  }

  #[test]
  fn bandwidth_zero_capacity_always_saturated() {
    let mut snap = BandwidthSnapshot {
      prev_bytes_sent: 0,
      prev_timestamp_ms: 1_000,
      current_throughput_bps: 0,
    };
    let est = evaluate_bandwidth(&mut snap, 1_000, 2_000, 0);
    assert_eq!(est.utilization_percent, 100);
    assert!(est.is_saturated);
  }
}
