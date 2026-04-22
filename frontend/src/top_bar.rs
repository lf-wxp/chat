//! Top bar component.

use crate::i18n;
use crate::state::use_app_state;
use icondata as i;
use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use leptos_icons::Icon;

/// Top bar component.
#[component]
pub fn TopBar() -> impl IntoView {
  let app_state = use_app_state();
  let i18n = i18n::use_i18n();
  let connected = app_state.connected;
  let theme = app_state.theme;

  // Pre-computed class for the connection indicator.
  let status_dot_class = move || {
    if connected.get() {
      "status-dot status-dot-online"
    } else {
      "status-dot status-dot-offline"
    }
  };

  view! {
    <header class="top-bar" data-testid="top-bar">
      // Left: connection status + title
      <div class="top-bar-left">
        <span class=status_dot_class role="status" aria-live="polite"></span>
        <h2 class="top-bar-title">{t!(i18n, app.title)}</h2>
      </div>

      // Right: theme toggle (icon cycles through the three modes)
      <div class="top-bar-right">
        <button
          class="btn-icon top-bar-btn"
          aria-label=move || t_string!(i18n, settings.theme)
          title=move || t_string!(i18n, settings.theme)
          on:click=move |_| {
            let current = theme.get();
            let new_theme = match current.as_str() {
              "light" => "dark",
              "dark" => "system",
              _ => "light",
            };
            theme.set(new_theme.to_string());
            if let Some(window) = web_sys::window()
              && let Ok(Some(storage)) = window.local_storage()
            {
              let _ = storage.set_item("theme", new_theme);
            }
          }
        >
          {move || match theme.get().as_str() {
            "light" => view! { <Icon icon=i::LuSun /> }.into_any(),
            "dark" => view! { <Icon icon=i::LuMoon /> }.into_any(),
            _ => view! { <Icon icon=i::LuMonitor /> }.into_any(),
          }}
        </button>
      </div>
    </header>
  }
}
