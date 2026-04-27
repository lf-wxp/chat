//! Timer helpers for [`super::CallManager`] (P2-New-1 split).
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
        // H4 fix: skip samples whose underlying `getStats()` report
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
        manager.evaluate_quality(worst).await;
      }
    });
  }

  /// Feed a classified sample into the hysteresis controller and
  /// apply the recommended video profile (Req 3.8c).
  ///
  /// When the quality drops to `Poor`, a one-shot toast is emitted
  /// (Req 14.10 — UX-1).
  pub(super) async fn evaluate_quality(&self, quality: message::types::NetworkQuality) {
    let action = self.inner.borrow_mut().quality.observe(quality);
    if let QualityAction::Apply(profile) = action {
      // P2-New-6 fix: expose the current video profile to the UI so
      // components can display resolution info or warn when degraded.
      self.signals.self_video_profile.set(profile);
      // UX-1: emit a one-shot toast when the profile drops to Poor
      // (Req 14.10). Uses AV201 — the only AV-status code currently
      // defined for advisory notices outside the device-permission
      // range (AV401-AV405).
      if profile == super::VideoProfile::VERY_LOW
        && let Some(toast) = self.error_toast.get()
      {
        toast.show_info_message_with_key("AV201", "call.network_poor", "");
      }
      if let Err(e) = self.apply_video_profile(profile).await {
        web_sys::console::warn_1(&format!("[call] applyConstraints failed: {e}").into());
      }
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
