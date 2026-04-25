//! Reconnect banner component.

use crate::i18n;
use crate::state::{RecoveryPhase, use_app_state};
use leptos::prelude::*;
use leptos_i18n::t_string;

/// Reconnect banner component (P2-1 fix, Req 10.11.40).
///
/// Only visible when the user is authenticated and the signaling client
/// is actively reconnecting. Checking `reconnecting` alone (rather than
/// also `!connected`) avoids a brief flash during normal logout where
/// `connected` is cleared before `auth` (P1 Bug-6 fix).
///
/// The banner text differs based on `recovery_phase`:
/// - `Reconnecting` → "Reconnecting..."
/// - `RestoringConnections` → "Restoring connections..."
#[component]
pub fn ReconnectBanner() -> impl IntoView {
  let app_state = use_app_state();
  let i18n = i18n::use_i18n();
  let auth = app_state.auth;
  let reconnecting = app_state.reconnecting;
  let recovery_phase = app_state.recovery_phase;

  view! {
    <Show when=move || auth.get().is_some() && reconnecting.get()>
      <div
        class="fixed top-0 left-0 right-0 z-toast flex items-center justify-center gap-2 px-4 py-2 bg-warning text-white shadow-lg"
        role="alert"
        aria-live="assertive"
      >
        <span class="text-sm">
          {move || match recovery_phase.get() {
            RecoveryPhase::RestoringConnections => t_string!(i18n, error.restoring_connections),
            RecoveryPhase::Reconnecting => t_string!(i18n, error.reconnecting),
          }}
        </span>
      </div>
    </Show>
  }
}
