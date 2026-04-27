//! Audio/video call subsystem.
//!
//! This module owns the call-specific half of the WebRTC mesh: the
//! finite-state machine that tracks whether the client is idle,
//! ringing, or in an active call; the local camera/microphone/display
//! capture pipeline; network-quality monitoring with a downgrade and
//! hysteresis-based recovery state machine; voice-activity detection
//! for the active-speaker indicator; and localStorage-backed refresh
//! recovery.
//!
//! ## Layering
//!
//! ```text
//!   UI   ──────────►  CallManager  ─┬──►  SignalingClient  (Call* msgs)
//!                      │            ├──►  WebRtcManager   (media tracks)
//!                      │            └──►  AppState        (reactive)
//!                      │
//!                      ├── media.rs   (getUserMedia / getDisplayMedia)
//!                      ├── stats.rs   (quality hysteresis)
//!                      ├── vad.rs     (voice activity detection)
//!                      └── types.rs   (CallState, LocalMediaState, ...)
//! ```
//!
//! The UI only ever talks to the `CallManager`. The manager, in turn,
//! delegates to the three collaborators wired up at bootstrap.

mod manager;
mod media;
mod notifier;
mod stats;
mod types;
mod vad;

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests;

pub use manager::{CallManager, CallSignals, INVITE_TIMEOUT_MS, RemoteParticipant, load_persisted};
pub use media::{
  acquire_display_stream, acquire_user_media, acquire_video_only_stream, attach_stream_to_video,
  exit_picture_in_picture, first_audio_track, first_video_track, request_picture_in_picture,
  retarget_video_track, stop_stream,
};
pub use stats::{QualityAction, QualityController, STATS_POLL_INTERVAL_MS, parse_stats_report};
pub use types::{
  CallEndReason, CallPhase, CallState, LocalMediaState, NetworkStatsSample, PersistedCallState,
  VideoProfile,
};
pub use vad::VoiceActivityDetector;

use leptos::prelude::*;

use crate::state::AppState;

/// Initialise the call subsystem and provide it via Leptos context.
///
/// The returned `CallManager` still needs to be wired up via
/// [`CallManager::set_signaling`] and [`CallManager::set_webrtc`]
/// before any user action reaches it; `lib.rs` performs the wiring
/// after the signaling and WebRTC managers are provided.
///
/// The toast manager is attached opportunistically from the current
/// Leptos context so the call-duration summary (Req 7.5) and other
/// user-visible notices do not need an extra wiring step in `lib.rs`.
#[must_use]
pub fn provide_call_manager(app_state: AppState) -> CallManager {
  let signals = CallSignals::new();
  let manager = CallManager::new(app_state, signals);
  provide_context(signals);
  provide_context(manager.clone());
  if let Some(toast) = use_context::<crate::error_handler::ErrorToastManager>() {
    manager.set_error_toast(toast);
  }
  manager.try_start_recovery();
  manager
}

/// Fetch the `CallManager` from the Leptos context.
///
/// # Panics
/// Panics if [`provide_call_manager`] has not been called.
#[must_use]
pub fn use_call_manager() -> CallManager {
  expect_context::<CallManager>()
}

/// Fetch the `CallSignals` from the Leptos context (no manager needed).
///
/// # Panics
/// Panics if [`provide_call_manager`] has not been called.
#[must_use]
pub fn use_call_signals() -> CallSignals {
  expect_context::<CallSignals>()
}

/// Non-panicking variant of [`use_call_manager`].
#[must_use]
pub fn try_use_call_manager() -> Option<CallManager> {
  use_context::<CallManager>()
}
