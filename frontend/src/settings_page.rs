//! Settings page component (placeholder for Task 23).

use crate::i18n;
use crate::state::use_app_state;
use leptos::prelude::*;
use leptos_i18n::t;

/// Settings page component (placeholder for Task 23).
#[component]
pub fn SettingsPage() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let app_state = use_app_state();
  let theme = app_state.theme;
  let locale = app_state.locale;

  view! {
    <div class="flex flex-col h-full overflow-y-auto p-6" data-testid="settings-page">
      <h1 class="text-2xl font-bold mb-6">{t!(i18n, settings.title)}</h1>

      // Appearance
      <section class="mb-8" aria-labelledby="appearance-heading">
        <h2 id="appearance-heading" class="text-lg font-semibold mb-4">
          {t!(i18n, settings.appearance)}
        </h2>

        // Theme selector
        <div class="mb-4">
          <label class="label">{t!(i18n, settings.theme)}</label>
          <div class="flex gap-2 mt-1">
            <button
              class=move || if theme.get() == "light" { "btn-primary btn-sm" } else { "btn-ghost btn-sm" }
              on:click=move |_| { theme.set("light".to_string()); }
            >
              {t!(i18n, settings.theme_light)}
            </button>
            <button
              class=move || if theme.get() == "dark" { "btn-primary btn-sm" } else { "btn-ghost btn-sm" }
              on:click=move |_| { theme.set("dark".to_string()); }
            >
              {t!(i18n, settings.theme_dark)}
            </button>
            <button
              class=move || if theme.get() == "system" { "btn-primary btn-sm" } else { "btn-ghost btn-sm" }
              on:click=move |_| { theme.set("system".to_string()); }
            >
              {t!(i18n, settings.theme_system)}
            </button>
          </div>
        </div>

        // Language selector
        <div class="mb-4">
          <label class="label">{t!(i18n, settings.language)}</label>
          <div class="flex gap-2 mt-1">
            <button
              class=move || if locale.get() == "en" { "btn-primary btn-sm" } else { "btn-ghost btn-sm" }
              on:click=move |_| { locale.set("en".to_string()); }
            >
              "English"
            </button>
            <button
              class=move || if locale.get() == "zh-CN" { "btn-primary btn-sm" } else { "btn-ghost btn-sm" }
              on:click=move |_| { locale.set("zh-CN".to_string()); }
            >
              "中文"
            </button>
          </div>
        </div>
      </section>
    </div>
  }
}
