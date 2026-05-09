//! Offline banner component for PWA support.
//!
//! Detects browser online/offline status and displays a banner when
//! the user loses internet connectivity. Unlike the reconnect banner
//! (which tracks signaling WebSocket state), this component monitors
//! the browser's `navigator.onLine` property and `online`/`offline`
//! events.

use crate::i18n;
use crate::state::use_app_state;
use icondata as i;
use leptos::prelude::*;
use leptos_i18n::t_string;
use leptos_icons::Icon;

/// Offline banner component.
///
/// Visible when the browser reports offline status. Dismissable by
/// the user; auto-resets when connectivity is restored. Shows a
/// brief "back online" confirmation before hiding.
#[component]
pub fn OfflineBanner() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let app_state = use_app_state();
  let auth = app_state.auth;

  // Track browser online/offline status.
  let is_online = RwSignal::new(
    web_sys::window()
      .map(|w| w.navigator().on_line())
      .unwrap_or(true),
  );

  // Track whether the user has manually dismissed the banner.
  let dismissed = RwSignal::new(false);

  // Track whether we're showing the "back online" confirmation.
  let showing_online_confirm = RwSignal::new(false);

  // Periodically check navigator.onLine for status changes.
  // This is a reliable fallback since leptos::ev does not expose
  // typed descriptors for online/offline events and raw Closure
  // management with StoredValue is complex. The interval is lightweight
  // (every 3s) and only reads a boolean flag from the browser API.
  let _ = leptos_use::use_interval_fn(
    move || {
      let current = web_sys::window()
        .map(|w| w.navigator().on_line())
        .unwrap_or(true);
      is_online.set(current);
    },
    3000,
  );

  // When transitioning from offline to online, auto-reset dismissal
  // and briefly show a "back online" confirmation.
  let prev_online = RwSignal::new(is_online.get_untracked());
  leptos_use::use_interval_fn(
    move || {
      let current = is_online.get();
      let prev = prev_online.get();
      if !prev && current {
        // Offline → Online transition detected
        dismissed.set(false);
        showing_online_confirm.set(true);
        let _ = crate::utils::set_timeout_once(3000, move || showing_online_confirm.set(false));
      }
      prev_online.set(current);
    },
    3000,
  );

  // Show offline banner when: authenticated, browser is offline,
  // and user hasn't dismissed it.
  let show_offline = move || auth.get().is_some() && !is_online.get() && !dismissed.get();

  // Show online confirmation when: authenticated, browser is online,
  // and the confirmation timer is active.
  let show_online = move || auth.get().is_some() && is_online.get() && showing_online_confirm.get();

  view! {
    <Show when=show_offline>
      <div
        class="offline-banner offline-banner--offline"
        role="alert"
        aria-live="assertive"
      >
        <Icon icon=i::LuWifiOff attr:class="offline-banner-icon" />
        <span class="offline-banner-text">
          {move || t_string!(i18n, pwa.offline_title)}
        </span>
        <span class="offline-banner-message">
          {move || t_string!(i18n, pwa.offline_message)}
        </span>
        <button
          type="button"
          class="offline-banner-btn"
          aria-label=move || t_string!(i18n, error.dismiss_banner)
          on:click=move |_| {
            dismissed.set(true);
          }
        >
          {move || t_string!(i18n, error.dismiss_banner)}
        </button>
      </div>
    </Show>
    <Show when=show_online>
      <div
        class="offline-banner offline-banner--online"
        role="status"
        aria-live="polite"
      >
        <Icon icon=i::LuWifi attr:class="offline-banner-icon" />
        <span class="offline-banner-text">
          {move || t_string!(i18n, pwa.online_title)}
        </span>
        <span class="offline-banner-message">
          {move || t_string!(i18n, pwa.online_message)}
        </span>
      </div>
    </Show>
  }
}
