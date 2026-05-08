//! Tiny connection-state badge shown in the sidebar header (Req 14.10.7).
//!
//! When the user is **not** in an audio/video call, the 4-bar network
//! quality indicator is not relevant; instead the sidebar exposes a
//! small WiFi-style icon that reflects WebSocket connectivity.

use crate::i18n;
use crate::state::use_app_state;
use icondata as i;
use leptos::prelude::*;
use leptos_i18n::t_string;
use leptos_icons::Icon;

/// Connection status icon — green when connected, amber while
/// reconnecting, red when disconnected.
#[component]
pub fn SidebarConnectionStatus() -> impl IntoView {
  let app_state = use_app_state();
  let i18n = i18n::use_i18n();

  // Derive a textual state so the tooltip and the CSS modifier class
  // stay in sync. `reconnecting` wins over `connected` because the
  // banner surface uses the same precedence (see ReconnectBanner).
  let state = Memo::new(move |_| {
    if app_state.reconnecting.get() {
      "reconnecting"
    } else if app_state.connected.get() {
      "connected"
    } else {
      "disconnected"
    }
  });

  let label = move || match state.get() {
    "reconnecting" => t_string!(i18n, common.reconnecting),
    "connected" => t_string!(i18n, common.connected),
    _ => t_string!(i18n, common.disconnected),
  };

  view! {
    <span
      class=move || format!("sidebar-connection-status sidebar-connection-status--{}", state.get())
      role="status"
      aria-live="polite"
      aria-label=label
      title=label
      data-testid="sidebar-connection-status"
    >
      {move || if state.get() == "disconnected" {
        view! { <Icon icon=i::LuWifiOff /> }.into_any()
      } else {
        view! { <Icon icon=i::LuWifi /> }.into_any()
      }}
    </span>
  }
}
