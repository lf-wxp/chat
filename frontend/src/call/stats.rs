//! Network-quality monitoring and the downgrade/restore state machine.
//!
//! The call subsystem polls `RTCPeerConnection.getStats()` every
//! `STATS_POLL_INTERVAL_MS` for every live peer, extracts RTT and
//! packet-loss metrics from the report, classifies them into a
//! 4-level [`NetworkQuality`] bucket, and feeds them into the
//! [`QualityController`] hysteresis machine below.
//!
//! The controller is implemented as a pure Rust state machine with no
//! dependency on `web_sys` so its behaviour can be verified with plain
//! unit tests (Req: "Write unit tests for call state machine, network
//! quality classification algorithm, downgrade/restore logic" from
//! task-18).

use js_sys::{Object, Reflect};
use message::types::NetworkQuality;
use wasm_bindgen::{JsCast, JsValue};

use super::types::{NetworkStatsSample, VideoProfile};

/// Poll interval for `getStats()` samples (5 s — Req 14.10).
pub const STATS_POLL_INTERVAL_MS: i32 = 5_000;

/// A fresh recovery requires at least this many consecutive "good"
/// samples (≥ `Good`) before the controller restores the full-quality
/// profile. At 5 s per sample this is ~10 s of sustained recovery.
const RECOVERY_REQUIRED_SAMPLES: u32 = 2;

/// Actions the controller can emit in response to a new quality sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityAction {
  /// No change; the current profile is still appropriate.
  Hold,
  /// Apply `profile` as the new outgoing video constraint because
  /// network conditions have changed.
  Apply(VideoProfile),
}

/// Hysteresis controller that turns a stream of [`NetworkQuality`]
/// samples into apply/hold decisions for the outgoing video profile.
///
/// The goal is to avoid *oscillation*: a single transient spike should
/// not downgrade, and a single good sample after prolonged poor quality
/// should not immediately restore full resolution.
///
/// The rules are:
/// * A sample that is *worse* than the current quality downgrades
///   immediately. Users prefer a lower-quality stream to no stream at
///   all, so we bias toward fast degradation.
/// * A sample that is *better* than the current quality increments a
///   recovery counter. Only when the counter reaches
///   `RECOVERY_REQUIRED_SAMPLES` do we upgrade one step.
/// * Any sample that is *not* better than the current quality resets
///   the recovery counter.
#[derive(Debug, Clone)]
pub struct QualityController {
  current: NetworkQuality,
  applied_profile: VideoProfile,
  /// Consecutive samples strictly better than `current`.
  recovery_streak: u32,
}

impl QualityController {
  /// Create a controller that starts optimistic at `Excellent` quality
  /// with the matching `HIGH` profile already applied.
  #[must_use]
  pub fn new() -> Self {
    Self::with_initial(NetworkQuality::Excellent)
  }

  /// Create a controller whose initial quality is pinned to a specific
  /// bucket. Used by unit tests to force a starting condition.
  #[must_use]
  pub fn with_initial(initial: NetworkQuality) -> Self {
    Self {
      current: initial,
      applied_profile: VideoProfile::for_quality(initial),
      recovery_streak: 0,
    }
  }

  /// The currently-applied video profile.
  #[must_use]
  pub const fn applied_profile(&self) -> VideoProfile {
    self.applied_profile
  }

  /// The currently-observed (debounced) quality level.
  #[must_use]
  pub const fn current_quality(&self) -> NetworkQuality {
    self.current
  }

  /// Consume a new raw sample and return what action, if any, the caller
  /// should take on the outgoing video track.
  #[must_use]
  pub fn observe(&mut self, sample: NetworkQuality) -> QualityAction {
    let sample_rank = quality_rank(sample);
    let current_rank = quality_rank(self.current);

    if sample_rank < current_rank {
      // Degradation — apply immediately.
      self.current = sample;
      self.recovery_streak = 0;
      let new_profile = VideoProfile::for_quality(sample);
      if new_profile == self.applied_profile {
        QualityAction::Hold
      } else {
        self.applied_profile = new_profile;
        QualityAction::Apply(new_profile)
      }
    } else if sample_rank > current_rank {
      // Potential recovery — require sustained improvement.
      self.recovery_streak = self.recovery_streak.saturating_add(1);
      if self.recovery_streak >= RECOVERY_REQUIRED_SAMPLES {
        // Step up one bucket toward the sample quality so recovery is
        // gradual — e.g. Poor → Fair → Good → Excellent over multiple
        // recovery windows.
        let next = step_up(self.current);
        if next == self.current {
          QualityAction::Hold
        } else {
          self.current = next;
          self.recovery_streak = 0;
          let new_profile = VideoProfile::for_quality(next);
          if new_profile == self.applied_profile {
            QualityAction::Hold
          } else {
            self.applied_profile = new_profile;
            QualityAction::Apply(new_profile)
          }
        }
      } else {
        QualityAction::Hold
      }
    } else {
      // Same bucket — reset any in-flight recovery streak so a single
      // "sample equal to current" does not accumulate toward a recovery
      // that is no longer justified.
      self.recovery_streak = 0;
      QualityAction::Hold
    }
  }
}

impl Default for QualityController {
  fn default() -> Self {
    Self::new()
  }
}

/// Rank quality so comparisons work ("higher rank = better").
///
/// Exposed `pub(crate)` so the call manager can compare per-peer
/// samples without re-implementing the same lookup table (P2-1 fix).
pub(crate) const fn quality_rank(q: NetworkQuality) -> u8 {
  match q {
    NetworkQuality::Poor => 0,
    NetworkQuality::Fair => 1,
    NetworkQuality::Good => 2,
    NetworkQuality::Excellent => 3,
  }
}

/// Step one bucket up (better) from a given quality.
const fn step_up(q: NetworkQuality) -> NetworkQuality {
  match q {
    NetworkQuality::Poor => NetworkQuality::Fair,
    NetworkQuality::Fair => NetworkQuality::Good,
    NetworkQuality::Good | NetworkQuality::Excellent => NetworkQuality::Excellent,
  }
}

/// Extract a single [`NetworkStatsSample`] from a raw
/// `RTCPeerConnection.getStats()` report.
///
/// The browser returns an `RTCStatsReport`, which is a JS `Map` whose
/// values are `RTCStats` dictionaries with a string `type` field. We
/// walk the map and look for:
///
/// * `candidate-pair` with `nominated: true` → `currentRoundTripTime`
///   (seconds, convert to ms).
/// * Any `inbound-rtp` entry → `packetsLost` / `packetsReceived` for
///   loss estimation. We sum across all inbound streams.
///
/// Returns `None` when the report contains *no* recognisable stats
/// (e.g. the connection is still being established or the browser
/// returned an empty map). H4 fix — the previous implementation
/// returned an "optimistic" zero-RTT / zero-loss sample which then
/// classified as `Excellent`, falsely advertising a perfect connection
/// to the user while the link was actually broken. Returning `None`
/// lets the caller skip the update so the UI surfaces "unknown" rather
/// than a misleading green bar.
#[must_use]
pub fn parse_stats_report(report: &JsValue, sampled_at_ms: i64) -> Option<NetworkStatsSample> {
  let mut rtt_ms: u64 = 0;
  let mut packets_lost: f64 = 0.0;
  let mut packets_received: f64 = 0.0;
  // Track which signal sources actually contributed so a wholly-empty
  // report can be distinguished from one that legitimately reports
  // zero RTT and zero loss.
  let mut saw_candidate_pair = false;
  let mut saw_inbound_rtp = false;

  // `RTCStatsReport` is spec'd as a JS Map. `Object::entries` gives us
  // `[key, value]` pairs for both Maps (via well-known symbol) and
  // plain objects, so it is the most portable walk strategy.
  let entries = Object::entries(report.unchecked_ref::<Object>());
  let len = entries.length();
  for i in 0..len {
    let entry = entries.get(i);
    let value = match Reflect::get(&entry, &JsValue::from_f64(1.0)) {
      Ok(v) => v,
      Err(_) => continue,
    };
    let Some(stat_type) = Reflect::get(&value, &JsValue::from_str("type"))
      .ok()
      .and_then(|v| v.as_string())
    else {
      continue;
    };

    match stat_type.as_str() {
      "candidate-pair" => {
        let nominated = Reflect::get(&value, &JsValue::from_str("nominated"))
          .ok()
          .and_then(|v| v.as_bool())
          .unwrap_or(false);
        if !nominated {
          continue;
        }
        saw_candidate_pair = true;
        if let Some(rtt_s) = Reflect::get(&value, &JsValue::from_str("currentRoundTripTime"))
          .ok()
          .and_then(|v| v.as_f64())
        {
          // Convert seconds → ms and keep the largest value seen on
          // any nominated pair (there is usually only one).
          let ms = (rtt_s * 1000.0).round().max(0.0);
          if ms as u64 > rtt_ms {
            rtt_ms = ms as u64;
          }
        }
      }
      "inbound-rtp" => {
        saw_inbound_rtp = true;
        if let Some(lost) = Reflect::get(&value, &JsValue::from_str("packetsLost"))
          .ok()
          .and_then(|v| v.as_f64())
        {
          packets_lost += lost.max(0.0);
        }
        if let Some(recv) = Reflect::get(&value, &JsValue::from_str("packetsReceived"))
          .ok()
          .and_then(|v| v.as_f64())
        {
          packets_received += recv.max(0.0);
        }
      }
      _ => {}
    }
  }

  // No recognised stats at all → treat the sample as unavailable so
  // the UI does not render a misleading "Excellent" indicator.
  if !saw_candidate_pair && !saw_inbound_rtp {
    return None;
  }

  let total = packets_lost + packets_received;
  let loss_percent = if total > 0.0 {
    (packets_lost / total) * 100.0
  } else {
    0.0
  };

  Some(NetworkStatsSample {
    rtt_ms,
    loss_percent,
    sampled_at_ms,
  })
}

#[cfg(test)]
mod tests;
