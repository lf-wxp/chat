//! Top bar component.

use crate::i18n;
use crate::state::use_app_state;
use icondata as i;
use leptos::prelude::*;
use leptos_i18n::t_string;
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

  // Derive the active conversation's display name for the header title.
  let active_conv_name = Signal::derive(move || {
    let conv_name = app_state.active_conversation.get().and_then(|id| {
      app_state.conversations.with_untracked(|convs| {
        convs
          .iter()
          .find(|c| c.id == id)
          .map(|c| c.display_name.clone())
      })
    });
    conv_name.unwrap_or_else(|| t_string!(i18n, app.title).to_string())
  });

  // Whether a conversation is currently active (for back button visibility).
  let has_conversation = Signal::derive(move || app_state.active_conversation.get().is_some());

  // Whether the sidebar is currently visible (for menu/close toggle).
  let sidebar_is_visible = Signal::derive(move || app_state.sidebar_visible.get());

  view! {
    <header class="top-bar" data-testid="top-bar">
      // Left: nav buttons (mobile) + connection status + title
      <div class="top-bar-left">
        // Mobile: back button when conversation active
        <Show when=move || has_conversation.get()>
          <button
            class="btn-icon top-bar-back-btn"
            aria-label=move || t_string!(i18n, common.back)
            title=move || t_string!(i18n, common.back)
            on:click=move |_| {
              app_state.active_conversation.set(None);
              app_state.sidebar_visible.set(false);
            }
          >
            <Icon icon=i::LuArrowLeft />
          </button>
        </Show>
        // Mobile: menu button to toggle sidebar (shown when no conversation
        // or when sidebar is closed)
        <Show when=move || !has_conversation.get() && !sidebar_is_visible.get()>
          <button
            class="btn-icon top-bar-menu-btn"
            aria-label=move || t_string!(i18n, common.menu)
            title=move || t_string!(i18n, common.menu)
            on:click=move |_| {
              app_state.sidebar_visible.set(true);
            }
          >
            <Icon icon=i::LuMenu />
          </button>
        </Show>
        <span class=status_dot_class role="status" aria-live="polite"></span>
        <h2 class="top-bar-title">{active_conv_name}</h2>
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
