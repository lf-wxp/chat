//! High-level call orchestration.
//!
//! The `CallManager` is the single source of truth for everything the
//! UI needs to render a call: current call state, local media state,
//! local preview stream, remote streams per peer, duration, active
//! speakers, and per-peer network quality. It plugs into three
//! collaborators provided at bootstrap:
//!
//! * `SignalingClient` — sends `Call*` messages to the server.
//! * `WebRtcManager`   — adds/replaces/removes media tracks on the
//!   existing mesh PeerConnections.
//! * `AppState`        — Leptos reactive signals observed by the UI.
//!
//! The manager is spread across multiple files to keep each focused:
//! * `mod.rs` (this file) — struct definitions, constructor, wiring,
//!   transition helper, shared constants.
//! * `lifecycle.rs` — `initiate_call` / `accept_call` / `decline_call`
//!   / `end_call`.
//! * `media_ops.rs` — `toggle_mute` / `toggle_camera` /
//!   `toggle_screen_share` / PiP / `apply_video_profile` / internal
//!   media helpers.
//! * `peer_events.rs` — incoming signaling handlers, peer connection
//!   events, DataChannel state broadcasts, refresh recovery.
//! * `timers.rs` — `arm_*`/`cancel_*` timer helpers.
//! * `persistence.rs` — localStorage serialisation.

mod lifecycle;
mod media_ops;
mod peer_events;
mod persistence;
mod timers;

pub use persistence::load_persisted;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use leptos::prelude::*;
use message::signaling::{CallAccept, CallDecline, CallEnd, CallInvite, SignalingMessage};
use message::types::MediaType;
use message::{RoomId, UserId};
use web_sys::MediaStream;

use crate::signaling::SignalingClient;
use crate::state::AppState;
use crate::utils::{IntervalHandle, TimeoutHandle};
use crate::webrtc::WebRtcManager;

use super::media;
use super::stats::{
  QualityAction, QualityController, STATS_POLL_INTERVAL_MS, parse_stats_report, quality_rank,
};
use super::types::{
  CallEndReason, CallPhase, CallState, LocalMediaState, NetworkStatsSample, PersistedCallState,
  VideoProfile,
};
use super::vad::VoiceActivityDetector;

/// Duration after which an un-answered invite automatically fails.
pub const INVITE_TIMEOUT_MS: i32 = 30_000;

/// Duration after which an unanswered incoming call (`Ringing`) is
/// auto-declined locally so the modal does not stay on screen if the
/// inviter crashes or loses connectivity (P1 Bug-4 fix). Spec'd at
/// 60 seconds to match Req 9.5's invitation behaviour.
pub const RINGING_TIMEOUT_MS: i32 = 60_000;

/// Duration-ticker cadence used to drive the call-bar clock.
pub(super) const DURATION_TICK_MS: i32 = 1_000;

/// VAD polling cadence. 10 Hz keeps the active-speaker indicator
/// responsive without dominating the main thread.
pub(super) const VAD_TICK_MS: i32 = 100;

/// A single remote participant's state as observed by the local client.
#[derive(Debug, Clone)]
pub struct RemoteParticipant {
  /// User id of the remote participant.
  pub user_id: UserId,
  /// Remote media stream, if we have received at least one track.
  pub stream: Option<MediaStream>,
  /// Whether VAD currently flags the participant as speaking.
  pub speaking: bool,
  /// Whether the participant is screen-sharing.
  pub screen_sharing: bool,
  /// Whether the participant's microphone is currently enabled
  /// (driven by `MediaStateUpdate` broadcasts — Req 3.5).
  pub mic_enabled: bool,
  /// Whether the participant's camera is currently enabled
  /// (driven by `MediaStateUpdate` broadcasts — Req 3.5).
  pub camera_enabled: bool,
  /// Whether the participant is currently reconnecting (driven by
  /// `ReconnectingState` broadcasts — Req 10.5.24).
  pub reconnecting: bool,
}

impl RemoteParticipant {
  fn new(user_id: UserId) -> Self {
    Self {
      user_id,
      stream: None,
      speaking: false,
      screen_sharing: false,
      mic_enabled: true,
      camera_enabled: true,
      reconnecting: false,
    }
  }
}

/// Reactive facet of the call subsystem exposed to the UI.
///
/// Grouped as a single `Copy` struct so components can accept it via
/// `CallSignals` rather than pulling 6 separate `RwSignal`s out of
/// `AppState`.
#[derive(Debug, Clone, Copy)]
pub struct CallSignals {
  /// Call finite-state machine.
  pub call_state: RwSignal<CallState>,
  /// Local microphone / camera / screen-share state.
  pub local_media: RwSignal<LocalMediaState>,
  /// Local capture preview (kept here rather than in `AppState` so
  /// the non-`Copy` `MediaStream` does not bloat every component).
  pub local_stream: RwSignal<Option<MediaStream>>,
  /// Remote participants keyed by user id.
  pub participants: RwSignal<HashMap<UserId, RemoteParticipant>>,
  /// Current call duration in seconds (updated every second).
  pub duration_secs: RwSignal<u64>,
  /// Picture-in-Picture mode active.
  pub pip_active: RwSignal<bool>,
  /// Whether a pending refresh-recovery prompt should be shown.
  pub recovery_prompt: RwSignal<Option<PersistedCallState>>,
  /// The video profile currently applied to the outgoing track by the
  /// quality controller (P2-New-6 fix). The UI can observe this to
  /// display the current capture resolution or warn when degraded.
  pub self_video_profile: RwSignal<VideoProfile>,
  /// Most-recent network stats sample per peer, so the UI can display
  /// RTT/loss details on hover (UX-2 fix, Req 14.10).
  pub network_stats: RwSignal<HashMap<UserId, NetworkStatsSample>>,
}

impl CallSignals {
  /// Create fresh reactive storage.
  #[must_use]
  pub fn new() -> Self {
    Self {
      call_state: RwSignal::new(CallState::Idle),
      local_media: RwSignal::new(LocalMediaState::off()),
      local_stream: RwSignal::new(None),
      participants: RwSignal::new(HashMap::new()),
      duration_secs: RwSignal::new(0),
      pip_active: RwSignal::new(false),
      recovery_prompt: RwSignal::new(None),
      self_video_profile: RwSignal::new(VideoProfile::HIGH),
      network_stats: RwSignal::new(HashMap::new()),
    }
  }
}

impl Default for CallSignals {
  fn default() -> Self {
    Self::new()
  }
}

/// Call-subsystem orchestrator.
///
/// Cloning this struct is cheap (`Rc` bumps only). All mutable state
/// lives behind the internal `RefCell`.
#[derive(Clone)]
pub struct CallManager {
  pub(super) app_state: AppState,
  pub(super) signaling: Rc<RefCell<Option<SignalingClient>>>,
  pub(super) webrtc: Rc<RefCell<Option<WebRtcManager>>>,
  /// Optional toast manager for user-visible notices (call duration
  /// summary on end — Req 7.5). Accessed via a `Cell` because
  /// `ErrorToastManager` is `Copy` and we want to update it at
  /// bootstrap without paying for a `RefCell`.
  pub(super) error_toast: Rc<Cell<Option<crate::error_handler::ErrorToastManager>>>,
  pub(super) signals: CallSignals,
  pub(super) inner: Rc<RefCell<Inner>>,
}

crate::wasm_send_sync!(CallManager);

pub(super) struct Inner {
  /// Periodic duration ticker, only alive while the call is `Active`.
  pub(super) duration_timer: Option<IntervalHandle>,
  /// One-shot timer that elapses [`INVITE_TIMEOUT_MS`] after `Inviting`
  /// entry. Stored as a [`TimeoutHandle`] (not `IntervalHandle`) so the
  /// callback fires exactly once and cannot leak into a phantom retry
  /// 30 s later (P0 Bug-2 fix).
  pub(super) invite_timeout: Option<TimeoutHandle>,
  /// One-shot timer that elapses [`RINGING_TIMEOUT_MS`] after `Ringing`
  /// entry. If the local user has not accepted/declined within the
  /// window we automatically transition to `Ended { InviteTimeout }`
  /// so the modal does not stay on screen forever when the inviter
  /// crashes mid-call (P1 Bug-4 fix).
  pub(super) ringing_timeout: Option<TimeoutHandle>,
  /// 5-second `getStats()` poll, armed on `Active` entry (Req 3.8a).
  pub(super) stats_timer: Option<IntervalHandle>,
  /// 100 ms VAD tick, armed on `Active` entry. Walks every peer with
  /// an installed [`VoiceActivityDetector`] and broadcasts a
  /// `set_peer_speaking` update (Req 3.7).
  pub(super) vad_timer: Option<IntervalHandle>,
  /// Per-peer VAD detectors; installed when a remote stream arrives
  /// and destroyed when the peer leaves or the call ends.
  pub(super) vad: HashMap<UserId, VoiceActivityDetector>,
  /// Hysteresis controller governing automatic video-profile
  /// downgrade/restoration (Req 3.8c).
  pub(super) quality: QualityController,
  /// Guard flag preventing re-entrant screen-share toggles (P2-4 fix).
  /// Set to `true` while a `toggle_screen_share` async path is in
  /// progress so the `onended` callback does not fire a second toggle
  /// concurrently.
  pub(super) screen_share_switching: Cell<bool>,
}

impl CallManager {
  /// Construct a new manager. The companion dependencies are set
  /// later via [`Self::set_signaling`] and [`Self::set_webrtc`].
  #[must_use]
  pub fn new(app_state: AppState, signals: CallSignals) -> Self {
    Self {
      app_state,
      signaling: Rc::new(RefCell::new(None)),
      webrtc: Rc::new(RefCell::new(None)),
      error_toast: Rc::new(Cell::new(None)),
      signals,
      inner: Rc::new(RefCell::new(Inner {
        duration_timer: None,
        invite_timeout: None,
        ringing_timeout: None,
        stats_timer: None,
        vad_timer: None,
        vad: HashMap::new(),
        quality: QualityController::new(),
        screen_share_switching: Cell::new(false),
      })),
    }
  }

  /// Attach the toast manager used for user-visible notices such as
  /// the call-duration summary (Req 7.5). Optional: if unset, the
  /// subsystem silently skips the toast calls.
  pub fn set_error_toast(&self, toast: crate::error_handler::ErrorToastManager) {
    self.error_toast.set(Some(toast));
  }

  /// Attach the signaling client. Must be called before any user
  /// action reaches the manager.
  pub fn set_signaling(&self, signaling: SignalingClient) {
    *self.signaling.borrow_mut() = Some(signaling);
  }

  /// Attach the WebRTC manager. Must be called before any user
  /// action reaches the manager.
  ///
  /// Also installs bridges for remote-track arrival, peer-connected
  /// and peer-closed events so mid-call peer joins see the current
  /// local stream (Task 18 — P2-3), and so peer drops drive the
  /// `AllPeersLeft` end reason (P1 Bug-5).
  pub fn set_webrtc(&self, webrtc: WebRtcManager) {
    let manager_for_track = self.clone();
    webrtc.set_on_remote_track(move |peer_id, stream| {
      manager_for_track.on_remote_stream(peer_id, stream);
    });
    let manager_for_close = self.clone();
    webrtc.set_on_peer_closed(move |peer_id| {
      manager_for_close.on_peer_closed(peer_id);
    });
    let manager_for_connect = self.clone();
    webrtc.set_on_peer_connected(move |peer_id| {
      manager_for_connect.on_peer_connected(peer_id);
    });
    // Req 3.5 / 7.1: consume peer media-state broadcasts so tiles show
    // muted / camera-off icons.
    let manager_for_media = self.clone();
    webrtc.set_on_media_state_update(move |peer_id, update| {
      manager_for_media.on_remote_media_state(peer_id, update);
    });
    // Req 10.5.24: consume peer reconnecting-status broadcasts so tiles
    // show a "reconnecting" hint while the peer recovers.
    let manager_for_reconnect = self.clone();
    webrtc.set_on_reconnecting_state(move |peer_id, state| {
      manager_for_reconnect.on_remote_reconnecting(peer_id, state);
    });
    *self.webrtc.borrow_mut() = Some(webrtc);
  }

  /// Reactive signals surface exposed to the UI layer.
  #[must_use]
  pub const fn signals(&self) -> CallSignals {
    self.signals
  }

  // ── Internals ───────────────────────────────────────────────────

  pub(super) fn transition(&self, next: CallState) {
    // Req 7.5 — when we leave an `Active` call, surface a duration
    // summary toast so the user sees how long the call lasted. We
    // deliberately compute the duration from the *pre-transition*
    // state so the toast remains accurate even if the terminating
    // branch has already called `tear_down_local_media`.
    if matches!(next, CallState::Ended { .. }) {
      self.maybe_emit_duration_toast();
    }
    self.signals.call_state.set(next);
  }

  fn send_signal(&self, msg: SignalingMessage) {
    if let Some(client) = self.signaling.borrow().as_ref()
      && let Err(e) = client.send(&msg)
    {
      web_sys::console::warn_1(
        &format!("[call] Failed to send {:?}: {e}", msg.discriminator()).into(),
      );
    }
  }

  /// Best-effort local user id, used to populate the `from` field on
  /// outgoing `Call*` signaling messages. The server overwrites this
  /// value with the authenticated user id, so a `nil` fallback only
  /// affects local rendering before auth completes.
  fn local_user_id(&self) -> UserId {
    self
      .app_state
      .auth
      .with_untracked(|auth| auth.as_ref().map(|a| a.user_id.clone()))
      .unwrap_or_else(|| UserId::from_uuid(uuid::Uuid::nil()))
  }
}

/// Unix ms via `Date.now()`.
pub(super) fn now_ms() -> i64 {
  js_sys::Date::now() as i64
}

#[cfg(test)]
mod tests;
