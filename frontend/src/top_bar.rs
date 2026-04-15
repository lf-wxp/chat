//! Top bar component.

use crate::i18n;
use crate::state::use_app_state;
use leptos::prelude::*;
use leptos_i18n::{t, t_string};

/// Top bar component.
#[component]
pub fn TopBar() -> impl IntoView {
  let app_state = use_app_state();
  let i18n = i18n::use_i18n();
  let connected = app_state.connected;

  view! {
    <header class="flex items-center justify-between px-4 py-2 border-b" data-testid="top-bar">
      // Connection status indicator
      <div class="flex items-center gap-2">
        <span
          class=move || if connected.get() { "text-success" } else { "text-error" }
          role="status"
        >
          {move || if connected.get() { "●" } else { "○" }}
        </span>
        <h2 class="text-sm font-semibold">{t!(i18n, app.title)}</h2>
      </div>

      // Right side controls
      <div class="flex items-center gap-2">
        // Theme toggle
        <button
          class="btn-icon"
          aria-label=move || t_string!(i18n, settings.theme)
          on:click=move |_| {
            let current = app_state.theme.get();
            let new_theme = match current.as_str() {
              "light" => "dark",
              "dark" => "system",
              _ => "light",
            };
            app_state.theme.set(new_theme.to_string());
            if let Some(window) = web_sys::window()
              && let Ok(Some(storage)) = window.local_storage()
            {
              let _ = storage.set_item("theme", new_theme);
            }
          }
        >
          {move || match app_state.theme.get().as_str() {
            "light" => "Light",
            "dark" => "Dark",
            _ => "System",
          }}
        </button>
      </div>
    </header>
  }
}
