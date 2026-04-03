//! Settings page and 404 page

use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use leptos_router::NavigateOptions;
use leptos_router::hooks::use_navigate;

use crate::{
  components::{Button, ButtonVariant, EmptyState},
  i18n::*,
  state,
};

/// Settings page component
#[component]
pub fn Settings() -> impl IntoView {
  let theme_state = state::use_theme_state();
  let user_state = state::use_user_state();
  let i18n = use_i18n();

  view! {
    <div class="page-settings">
      <div class="settings-card">
        <h2>{t!(i18n, settings_title)}</h2>

        // Theme settings
        <div class="settings-section">
          <h3>{t!(i18n, settings_theme)}</h3>
          <div class="settings-options">
            <button
              class=move || format!("settings-option {}", if theme_state.get().theme == state::Theme::System { "active" } else { "" })
              on:click=move |_| theme_state.update(|s| s.theme = state::Theme::System)
              tabindex=0
            >
              "🌐 " {t!(i18n, settings_theme_system)}
            </button>
            <button
              class=move || format!("settings-option {}", if theme_state.get().theme == state::Theme::Light { "active" } else { "" })
              on:click=move |_| theme_state.update(|s| s.theme = state::Theme::Light)
              tabindex=0
            >
              "☀️ " {t!(i18n, settings_theme_light)}
            </button>
            <button
              class=move || format!("settings-option {}", if theme_state.get().theme == state::Theme::Dark { "active" } else { "" })
              on:click=move |_| theme_state.update(|s| s.theme = state::Theme::Dark)
              tabindex=0
            >
              "🌙 " {t!(i18n, settings_theme_dark)}
            </button>
          </div>
        </div>

        // User information
        <div class="settings-section">
          <h3>{t!(i18n, settings_account)}</h3>
          <div class="settings-info">
            <span class="text-secondary">{t!(i18n, settings_username_label)}</span>
            <span class="font-medium">{move || user_state.get().username.clone()}</span>
          </div>
          <Button
            label=t_string!(i18n, settings_logout).to_string()
            variant=ButtonVariant::Danger
            on_click=Callback::new(move |()| {
              user_state.update(|s| {
                s.authenticated = false;
                s.token.clear();
              });
              let navigate = use_navigate();
              navigate("/login", NavigateOptions::default());
            })
          />
        </div>
      </div>
    </div>
  }
}

/// 404 Not Found page
#[component]
pub fn NotFound() -> impl IntoView {
  let i18n = use_i18n();

  view! {
    <div class="page-not-found">
      <EmptyState
        icon="🔍"
        title=t_string!(i18n, common_404_title).to_string()
        description=t_string!(i18n, common_404_desc).to_string()
      />
    </div>
  }
}
