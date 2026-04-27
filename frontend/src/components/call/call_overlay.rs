//! Top-level call overlay mounted once by the app shell.
//!
//! Dispatches between the three call-UI surfaces based on the current
//! [`crate::call::CallState`] and the optional refresh-recovery prompt:
//!
//! * `CallRecoveryPrompt` — shown on bootstrap when a persisted call
//!   was found in localStorage.
//! * `IncomingCallModal` — shown whenever `CallState::Ringing`.
//! * `CallView` — shown for `CallState::Inviting` and `CallState::Active`.

use leptos::prelude::*;

use crate::components::call::{CallRecoveryPrompt, CallView, IncomingCallModal};

/// Root-level call overlay component.
#[component]
pub fn CallOverlay() -> impl IntoView {
  view! {
    <CallRecoveryPrompt />
    <IncomingCallModal />
    <CallView />
  }
}
