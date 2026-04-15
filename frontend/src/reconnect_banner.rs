//! Reconnect banner component.

use crate::i18n;
use crate::state::use_app_state;
use leptos::prelude::*;
use leptos_i18n::t;

/// Reconnect banner component.
#[component]
pub fn ReconnectBanner() -> impl IntoView {
  let app_state = use_app_state();
  let i18n = i18n::use_i18n();
  let reconnecting = app_state.reconnecting;
  let connected = app_state.connected;

  view! {
    <Show when=move || !connected.get() || reconnecting.get()>
      <div
        class="fixed bottom-4 left-1/2 -translate-x-1/2 z-toast flex items-center gap-2 px-4 py-2 rounded-lg bg-warning text-white shadow-lg"
        role="alert"
        aria-live="assertive"
      >
        <span class="text-sm">{t!(i18n, error.reconnecting)}</span>
      </div>
    </Show>
  }
}
