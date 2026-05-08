//! Reconnect banner component.

use crate::i18n;
use crate::signaling::SignalingClient;
use crate::state::{RecoveryPhase, use_app_state};
use leptos::prelude::*;
use leptos_i18n::t_string;

/// Reconnect banner component (Req 10.11.40).
///
/// Only visible when the user is authenticated and the signaling client
/// is actively reconnecting. Checking `reconnecting` alone (rather than
/// also `!connected`) avoids a brief flash during normal logout where
/// `connected` is cleared before `auth`.
///
/// The banner text differs based on `recovery_phase`:
/// - `Reconnecting` → "Reconnecting..."
/// - `RestoringConnections` → "Restoring connections..."
///
/// Users can dismiss the banner or retry the connection manually
/// (Req 14.5.2 — interactive banners).
#[component]
pub fn ReconnectBanner() -> impl IntoView {
  let app_state = use_app_state();
  let i18n = i18n::use_i18n();
  let auth = app_state.auth;
  let reconnecting = app_state.reconnecting;
  let recovery_phase = app_state.recovery_phase;

  // Track whether the user has manually dismissed the banner during
  // the current reconnection attempt. Reset automatically when
  // `reconnecting` transitions back to `false` so the banner
  // reappears on the next outage.
  let dismissed = RwSignal::new(false);

  // Auto-clear `dismissed` when reconnection succeeds so the banner
  // is visible again on the next disconnection.
  Effect::new(move |_| {
    if !reconnecting.get() {
      dismissed.set(false);
    }
  });

  let show = move || auth.get().is_some() && reconnecting.get() && !dismissed.get();

  view! {
    <Show when=show>
      <div
        class="reconnect-banner"
        role="status"
        aria-live="polite"
      >
        <span class="reconnect-banner-text">
          {move || match recovery_phase.get() {
            RecoveryPhase::RestoringConnections => t_string!(i18n, error.restoring_connections),
            RecoveryPhase::Reconnecting => t_string!(i18n, error.reconnecting),
          }}
        </span>
        // Retry button — triggers a fresh reconnection attempt via
        // the signaling client's explicit `force_reconnect_now()`
        // API. Falls back to the legacy "reconnecting flap" pattern
        // when the SignalingClient context is unavailable (e.g.
        // during early bootstrap or in an isolated component test).
        <button
          type="button"
          class="reconnect-banner-btn"
          aria-label=move || t_string!(i18n, error.retry_connection)
          on:click=move |_| {
            if let Some(client) = use_context::<SignalingClient>() {
              client.force_reconnect_now();
            } else {
              reconnecting.set(false);
              reconnecting.set(true);
            }
          }
        >
          {move || t_string!(i18n, error.retry)}
        </button>
        // Dismiss button — hides the banner without affecting the
        // reconnection state machine.
        <button
          type="button"
          class="reconnect-banner-btn"
          aria-label=move || t_string!(i18n, error.dismiss_banner)
          on:click=move |_| {
            dismissed.set(true);
          }
        >
          {move || t_string!(i18n, error.dismiss_banner)}
        </button>
      </div>
    </Show>
  }
}
