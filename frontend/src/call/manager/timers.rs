//! Timer helpers for [`super::CallManager`].
//!
//! Groups the `arm_*`/`cancel_*` methods that were previously inlined
//! in `manager/mod.rs`. Nothing here owns business logic — the timers
//! fire callbacks that delegate back to `CallManager`'s state-machine
//! methods. Splitting them into a sibling file keeps the main module
//! focused on lifecycle, media, and recovery.

use leptos::prelude::*;
use leptos::task::spawn_local;

use super::{
  CallEndReason, CallManager, CallState, DURATION_TICK_MS, INVITE_TIMEOUT_MS, QualityAction,
  RINGING_TIMEOUT_MS, STATS_POLL_INTERVAL_MS, VAD_TICK_MS, now_ms, parse_stats_report,
  quality_rank,
};
use crate::utils::{set_interval, set_timeout_once};
use message::UserId;

/// Minimum interval between two consecutive "Network quality is poor"
/// toasts (Req 14.10.4). A flapping link should not surface the same
/// warning more than twice per minute.
pub(super) const POOR_TOAST_THROTTLE_MS: i64 = 30_000;

/// Pure decision helper for the Poor-toast throttle. Returns `true`
/// when the toast should be emitted given the wall-clock instant
/// `now_ms` and the timestamp of the last emission.
///
/// Exposed as a free function so the time-window arithmetic can be
/// exercised by unit tests without a live `Inner`.
#[must_use]
pub(super) fn should_emit_poor_toast(now_ms: i64, last_ms: Option<i64>) -> bool {
  match last_ms {
    None => true,
    Some(prev) => now_ms.saturating_sub(prev) >= POOR_TOAST_THROTTLE_MS,
  }
}

/// Pure decision helper for the Poor → recovered transition.
/// Returns `true` when the controller should emit the
/// "Network quality restored" toast (Req 14.10.5):
/// the previous classification was Poor and the current one is
/// Good or Excellent. Fair is intentionally excluded so the toast
/// only fires on a meaningful recovery, not a one-step improvement.
#[must_use]
pub(super) fn should_emit_recovery_toast(
  was_poor: bool,
  current: message::types::NetworkQuality,
) -> bool {
  use message::types::NetworkQuality::{Excellent, Good};
  was_poor && matches!(current, Good | Excellent)
}

impl CallManager {
  /// Arm the one-shot invite timeout ([`INVITE_TIMEOUT_MS`]). On expiry
  /// transitions `Inviting → Ended { InviteTimeout }` and cleans up
  /// local media.
  pub(super) fn arm_invite_timeout(&self) {
    self.cancel_invite_timeout();
    let manager = self.clone();
    let handle = set_timeout_once(INVITE_TIMEOUT_MS, move || {
      if matches!(
        manager.signals.call_state.get_untracked(),
        CallState::Inviting { .. }
      ) {
        manager.tear_down_local_media();
        manager.transition(CallState::Ended {
          reason: CallEndReason::InviteTimeout,
        });
        manager.cancel_timers();
        manager.clear_persist();
      }
    });
    if let Some(h) = handle {
      self.inner.borrow_mut().invite_timeout = Some(h);
    }
  }

  pub(super) fn cancel_invite_timeout(&self) {
    let handle = self.inner.borrow_mut().invite_timeout.take();
    if let Some(h) = handle {
      h.cancel();
    }
  }

  /// Arm the [`RINGING_TIMEOUT_MS`] one-shot. When it fires we treat
  /// the call as locally declined (no `CallDecline` sent — the inviter
  /// likely already gave up and we do not want to surface a stale
  /// "declined" toast on their side).
  pub(super) fn arm_ringing_timeout(&self) {
    self.cancel_ringing_timeout();
    let manager = self.clone();
    let handle = set_timeout_once(RINGING_TIMEOUT_MS, move || {
      if matches!(
        manager.signals.call_state.get_untracked(),
        CallState::Ringing { .. }
      ) {
        manager.transition(CallState::Ended {
          reason: CallEndReason::InviteTimeout,
        });
        manager.cancel_timers();
        manager.clear_persist();
      }
    });
    if let Some(h) = handle {
      self.inner.borrow_mut().ringing_timeout = Some(h);
    }
  }

  pub(super) fn cancel_ringing_timeout(&self) {
    let handle = self.inner.borrow_mut().ringing_timeout.take();
    if let Some(h) = handle {
      h.cancel();
    }
  }

  /// Arm the 1 Hz ticker that drives the call-bar clock.
  pub(super) fn arm_duration_ticker(&self) {
    self.cancel_duration_ticker();
    let signals = self.signals;
    let handle = set_interval(DURATION_TICK_MS, move || {
      if let Some(started) = signals.call_state.get_untracked().active_started_at_ms() {
        let elapsed = now_ms().saturating_sub(started).max(0);
        let secs = (elapsed / 1000) as u64;
        signals.duration_secs.set(secs);
      }
    });
    if let Some(h) = handle {
      self.inner.borrow_mut().duration_timer = Some(h);
    }
  }

  pub(super) fn cancel_duration_ticker(&self) {
    let handle = self.inner.borrow_mut().duration_timer.take();
    if let Some(h) = handle {
      h.cancel();
    }
  }

  /// Start the `getStats()` poller (Req 3.8a). Samples every
  /// [`STATS_POLL_INTERVAL_MS`], parses RTT + loss, classifies into a
  /// [`message::types::NetworkQuality`] bucket, updates the per-peer
  /// app-state signal, and feeds the hysteresis controller so we can
  /// adjust the outgoing video profile automatically.
  pub(super) fn arm_stats_poller(&self) {
    self.cancel_stats_poller();
    let manager = self.clone();
    let handle = set_interval(STATS_POLL_INTERVAL_MS, move || {
      manager.spawn_stats_sweep();
    });
    if let Some(h) = handle {
      self.inner.borrow_mut().stats_timer = Some(h);
    }
  }

  pub(super) fn cancel_stats_poller(&self) {
    let handle = self.inner.borrow_mut().stats_timer.take();
    if let Some(h) = handle {
      h.cancel();
    }
  }

  /// Collect a stats sweep asynchronously without blocking the
  /// interval callback.
  pub(super) fn spawn_stats_sweep(&self) {
    let Some(webrtc) = self.webrtc.borrow().clone() else {
      return;
    };
    let manager = self.clone();
    spawn_local(async move {
      let reports = webrtc.collect_stats().await;
      let now = now_ms();
      // Track the *worst* quality seen across every live peer so the
      // local outgoing profile always accommodates the most-degraded
      // remote listener. Round-4 rename: the variable was previously
      // named `best_quality`, which confused readers because the
      // aggregation rule is strictly "lower rank wins".
      let mut worst_quality: Option<message::types::NetworkQuality> = None;
      for (peer_id, report) in reports {
        // skip samples whose underlying `getStats()` report
        // contained no recognisable entries. Folding these into the
        // aggregate would bias the worst-quality calculation toward
        // an artificial "Excellent" reading and falsely advertise a
        // healthy link to the user.
        let Some(sample) = parse_stats_report(&report, now) else {
          continue;
        };
        let classified = sample.classify();
        match worst_quality {
          None => worst_quality = Some(classified),
          Some(current) if quality_rank(classified) < quality_rank(current) => {
            worst_quality = Some(classified);
          }
          _ => {}
        }
        manager.on_network_sample(peer_id, sample);
      }
      if let Some(worst) = worst_quality {
        // Mirror the aggregate quality onto the local user id so the
        // local video tile's NetworkIndicator can display the user's
        // own connection state (Req 14.10.6 — review v3 §R5). The
        // local id is sourced from auth state; if the user is not
        // authenticated we skip the write — the tile will fall back
        // to the "Unknown" state.
        if let Some(local_id) = manager.app_state.current_user_id() {
          manager.app_state.network_quality.update(|map| {
            map.insert(local_id, worst);
          });
        }
        manager.evaluate_quality(worst).await;
      }
    });
  }

  /// Feed a classified sample into the hysteresis controller and
  /// apply the recommended video profile (Req 3.8c).
  ///
  /// Network quality toasts (Req 14.10.4 / 14.10.5):
  /// * Poor → emit "network is poor", throttled to one per
  ///   [`POOR_TOAST_THROTTLE_MS`] (30 s).
  /// * Poor → Good/Excellent → emit "Network quality restored" once
  ///   per recovery edge.
  pub(super) async fn evaluate_quality(&self, quality: message::types::NetworkQuality) {
    let action = self.inner.borrow_mut().quality.observe(quality);

    // Detect Poor / recovery transitions on every sample, not only
    // when the hysteresis controller decides to apply a new profile.
    // This decouples user-visible feedback from the slower video-
    // profile changes (which require sustained samples to step up).
    self.maybe_emit_quality_toasts(quality);

    if let QualityAction::Apply(profile) = action {
      // expose the current video profile to the UI so
      // components can display resolution info or warn when degraded.
      self.signals.self_video_profile.set(profile);
      if let Err(e) = self.apply_video_profile(profile).await {
        web_sys::console::warn_1(&format!("[call] applyConstraints failed: {e}").into());
      }
    }
  }

  /// Surface user-visible toasts for the Poor / restored transitions.
  ///
  /// Updates the `last_poor_toast_ms` and `was_poor` state in `Inner`
  /// so subsequent calls observe the proper throttle / recovery edge.
  /// Uses error code `AV201` — the only AV-status code currently
  /// defined for advisory notices outside the device-permission
  /// range (AV401-AV405).
  ///
  /// `was_poor` is treated as a **sticky flag**: once set by a Poor
  /// sample it is only cleared when we actually emit the recovery
  /// toast. This guarantees the Poor → Fair → Good path still
  /// surfaces "Network quality restored" — the previous
  /// implementation reset the flag on the intermediate Fair sample
  /// and silently swallowed the recovery edge (review v3 §B2).
  fn maybe_emit_quality_toasts(&self, quality: message::types::NetworkQuality) {
    use message::types::NetworkQuality;
    let now = now_ms();
    let toast = self.error_toast.get();

    // Snapshot + decide while holding the borrow, then drop it before
    // calling into the toast manager (which may dispatch reactive
    // updates that re-enter `Inner`).
    let (emit_poor, emit_recovered) = {
      let mut inner = self.inner.borrow_mut();
      let was_poor = inner.was_poor;
      let current_is_poor = matches!(quality, NetworkQuality::Poor);

      let emit_poor = current_is_poor && should_emit_poor_toast(now, inner.last_poor_toast_ms);
      if emit_poor {
        inner.last_poor_toast_ms = Some(now);
      }

      let emit_recovered = should_emit_recovery_toast(was_poor, quality);

      // Sticky update: Poor sets the flag, recovery clears it,
      // intermediate states (Fair) keep the flag intact so a later
      // step up to Good/Excellent still emits the recovery toast.
      if current_is_poor {
        inner.was_poor = true;
      } else if emit_recovered {
        inner.was_poor = false;
      }

      (emit_poor, emit_recovered)
    };

    let Some(toast) = toast else {
      return;
    };
    if emit_poor {
      toast.show_info_message_with_key("AV201", "call.network_poor", "");
    }
    if emit_recovered {
      // Req 14.10.5: restored toast auto-dismisses after 2 s rather
      // than the default 8 s — recovery is a momentary acknowledgement,
      // not an actionable warning.
      toast.show_info_message_with_key_and_duration(
        "AV201",
        "call.quality_restored",
        "",
        crate::error_handler::QUALITY_RESTORED_DURATION_MS,
      );
    }
  }

  /// Arm the 10 Hz VAD tick (Req 3.7). Reads every installed detector
  /// and broadcasts the `speaking` flag via `set_peer_speaking`.
  pub(super) fn arm_vad_ticker(&self) {
    self.cancel_vad_ticker();
    let manager = self.clone();
    let handle = set_interval(VAD_TICK_MS, move || {
      manager.sweep_vad();
    });
    if let Some(h) = handle {
      self.inner.borrow_mut().vad_timer = Some(h);
    }
  }

  pub(super) fn cancel_vad_ticker(&self) {
    let handle = self.inner.borrow_mut().vad_timer.take();
    if let Some(h) = handle {
      h.cancel();
    }
  }

  pub(super) fn sweep_vad(&self) {
    // Collect results under a short borrow, then fan them out under a
    // separate one so we do not nest borrows while updating the
    // participants signal.
    let updates: Vec<(UserId, bool)> = {
      let mut inner = self.inner.borrow_mut();
      inner
        .vad
        .iter_mut()
        .map(|(peer, detector)| (peer.clone(), detector.is_speaking()))
        .collect()
    };
    for (peer, speaking) in updates {
      self.set_peer_speaking(&peer, speaking);
    }
  }

  /// Cancel every timer owned by the manager. Safe to call repeatedly.
  ///
  /// Safe to call from within a timer callback: the per-timer
  /// `cancel_*` helpers each take a short `inner.borrow_mut()` that is
  /// dropped before the next one runs, so even when an expiring
  /// `set_timeout_once` callback invokes this helper transitively (via
  /// e.g. `tear_down_local_media → cancel_timers`) no borrow is held
  /// across the nested calls. The `clear_timeout` issued by the canceler
  /// on the already-fired handle is a harmless no-op.
  pub(super) fn cancel_timers(&self) {
    self.cancel_invite_timeout();
    self.cancel_ringing_timeout();
    self.cancel_duration_ticker();
    self.cancel_stats_poller();
    self.cancel_vad_ticker();
  }

  /// Arm every timer that should live for the duration of an active
  /// call: duration ticker, `getStats()` poll, and VAD sweep.
  pub(super) fn arm_active_timers(&self) {
    self.arm_duration_ticker();
    self.arm_stats_poller();
    self.arm_vad_ticker();
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use message::types::NetworkQuality::{Excellent, Fair, Good, Poor};

  #[test]
  fn poor_toast_emits_when_no_previous_emission() {
    assert!(should_emit_poor_toast(0, None));
    assert!(should_emit_poor_toast(1_000_000, None));
  }

  #[test]
  fn poor_toast_throttled_within_window() {
    // 0.5 s after the previous Poor toast — still inside the 30 s
    // throttle window, so the helper must suppress.
    assert!(!should_emit_poor_toast(500, Some(0)));
    // Just before the window edge.
    assert!(!should_emit_poor_toast(POOR_TOAST_THROTTLE_MS - 1, Some(0)));
  }

  #[test]
  fn poor_toast_emits_at_or_after_window_edge() {
    // At the exact edge — emit (the requirement says "at most once
    // per 30 seconds", inclusive of the boundary).
    assert!(should_emit_poor_toast(POOR_TOAST_THROTTLE_MS, Some(0)));
    assert!(should_emit_poor_toast(POOR_TOAST_THROTTLE_MS + 1, Some(0)));
  }

  #[test]
  fn recovery_toast_only_after_poor() {
    // No previous Poor → no recovery toast even on Excellent.
    assert!(!should_emit_recovery_toast(false, Excellent));
    assert!(!should_emit_recovery_toast(false, Good));
    // Was Poor + now Good/Excellent → emit.
    assert!(should_emit_recovery_toast(true, Good));
    assert!(should_emit_recovery_toast(true, Excellent));
  }

  #[test]
  fn recovery_toast_skipped_for_partial_recovery() {
    // Poor → Fair is too modest a recovery; users don't get a green
    // "restored" message until quality is at least Good.
    assert!(!should_emit_recovery_toast(true, Fair));
    assert!(!should_emit_recovery_toast(true, Poor));
  }

  // ── B2: was_poor must be sticky across intermediate Fair samples ──
  //
  // The fix in `maybe_emit_quality_toasts` keeps `was_poor` set when a
  // sample is Fair so the subsequent Good/Excellent edge still emits
  // the recovery toast. We model the same logic here in a pure function
  // simulator so the regression cannot creep back in.
  fn next_was_poor(prev: bool, sample: message::types::NetworkQuality) -> bool {
    let current_is_poor = matches!(sample, Poor);
    let emit_recovered = should_emit_recovery_toast(prev, sample);
    if current_is_poor {
      true
    } else if emit_recovered {
      false
    } else {
      prev
    }
  }

  #[test]
  fn was_poor_is_sticky_through_fair() {
    // Start clean.
    let mut was_poor = false;
    // First Poor sample → flag becomes true.
    was_poor = next_was_poor(was_poor, Poor);
    assert!(was_poor);
    // Fair sample: should NOT emit recovery and must keep flag.
    assert!(!should_emit_recovery_toast(was_poor, Fair));
    was_poor = next_was_poor(was_poor, Fair);
    assert!(was_poor, "Fair must not clear the sticky Poor flag");
    // Good sample now triggers the recovery edge and clears the flag.
    assert!(should_emit_recovery_toast(was_poor, Good));
    was_poor = next_was_poor(was_poor, Good);
    assert!(!was_poor, "recovery toast must clear the sticky flag");
    // Subsequent Excellent samples must NOT re-emit recovery.
    assert!(!should_emit_recovery_toast(was_poor, Excellent));
  }

  #[test]
  fn was_poor_clears_only_on_good_or_excellent() {
    // Direct Poor → Excellent path also works.
    let mut was_poor = next_was_poor(false, Poor);
    assert!(was_poor);
    assert!(should_emit_recovery_toast(was_poor, Excellent));
    was_poor = next_was_poor(was_poor, Excellent);
    assert!(!was_poor);
  }

  #[test]
  fn was_poor_resets_on_recovery_then_re_engages_on_new_poor() {
    let mut was_poor = false;
    // Poor → Good → Poor cycle.
    was_poor = next_was_poor(was_poor, Poor);
    was_poor = next_was_poor(was_poor, Good);
    assert!(!was_poor);
    was_poor = next_was_poor(was_poor, Poor);
    assert!(was_poor, "new Poor sample must re-arm the sticky flag");
  }
}
