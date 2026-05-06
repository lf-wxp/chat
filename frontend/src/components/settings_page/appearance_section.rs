//! Appearance settings (theme / language / font size).
//!
//! Consumers of this component do not need any props: it binds
//! directly to the `AppState` theme / locale signals and the
//! settings-store font scale.

use super::class_helpers::segmented_item_class;
use crate::i18n;
use crate::settings::{FontScale, use_settings_state};
use crate::state::use_app_state;
use icondata as i;
use leptos::prelude::*;
use leptos_i18n::t;
use leptos_icons::Icon;

/// Theme / language / font size section.
#[component]
pub fn AppearanceSection() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let app_state = use_app_state();
  let settings = use_settings_state();
  let theme = app_state.theme;
  let locale = app_state.locale;

  // Local derived signals avoid borrowing `settings` inside each
  // reactive closure below.
  let font_scale = Memo::new(move |_| settings.get().font_scale);

  view! {
    <section class="settings-section" aria-labelledby="appearance-heading">
      <h2 id="appearance-heading" class="settings-section-title">
        <Icon icon=i::LuPalette attr:class="settings-section-icon" />
        {t!(i18n, settings.appearance)}
      </h2>

      // Theme selector
      <div class="settings-row">
        <label class="settings-label">
          <Icon icon=i::LuPalette attr:class="settings-label-icon" />
          {t!(i18n, settings.theme)}
        </label>
        <div class="segmented" role="group" data-testid="theme-group">
          <button
            class=move || segmented_item_class(theme.get() == "light")
            on:click=move |_| theme.set("light".to_string())
            aria-pressed=move || (theme.get() == "light").to_string()
          >
            <Icon icon=i::LuSun />
            <span>{t!(i18n, settings.theme_light)}</span>
          </button>
          <button
            class=move || segmented_item_class(theme.get() == "dark")
            on:click=move |_| theme.set("dark".to_string())
            aria-pressed=move || (theme.get() == "dark").to_string()
          >
            <Icon icon=i::LuMoon />
            <span>{t!(i18n, settings.theme_dark)}</span>
          </button>
          <button
            class=move || segmented_item_class(theme.get() == "system")
            on:click=move |_| theme.set("system".to_string())
            aria-pressed=move || (theme.get() == "system").to_string()
          >
            <Icon icon=i::LuMonitor />
            <span>{t!(i18n, settings.theme_system)}</span>
          </button>
        </div>
      </div>

      // Language selector
      <div class="settings-row">
        <label class="settings-label">
          <Icon icon=i::LuGlobe attr:class="settings-label-icon" />
          {t!(i18n, settings.language)}
        </label>
        <div class="segmented" role="group" data-testid="language-group">
          <button
            class=move || segmented_item_class(locale.get() == "en")
            on:click=move |_| locale.set("en".to_string())
            aria-pressed=move || (locale.get() == "en").to_string()
          >
            <span>"English"</span>
          </button>
          <button
            class=move || segmented_item_class(locale.get() == "zh-CN")
            on:click=move |_| locale.set("zh-CN".to_string())
            aria-pressed=move || (locale.get() == "zh-CN").to_string()
          >
            <span>"中文"</span>
          </button>
        </div>
      </div>

      // Font size selector
      <div class="settings-row">
        <label class="settings-label">
          <Icon icon=i::ImFont attr:class="settings-label-icon" />
          {t!(i18n, settings.font_size)}
        </label>
        <div class="segmented" role="group" data-testid="font-size-group">
          {[
            (FontScale::Small, "font-size-small"),
            (FontScale::Medium, "font-size-medium"),
            (FontScale::Large, "font-size-large"),
          ]
            .into_iter()
            .map(|(scale, testid)| {
              let label = match scale {
                FontScale::Small => t!(i18n, settings.font_size_small).into_any(),
                FontScale::Medium => t!(i18n, settings.font_size_medium).into_any(),
                FontScale::Large => t!(i18n, settings.font_size_large).into_any(),
              };
              view! {
                <button
                  class=move || segmented_item_class(font_scale.get() == scale)
                  on:click=move |_| {
                    settings.update(|s| s.font_scale = scale);
                  }
                  aria-pressed=move || (font_scale.get() == scale).to_string()
                  data-testid=testid
                >
                  <span>{label}</span>
                </button>
              }
            })
            .collect::<Vec<_>>()}
        </div>
      </div>
    </section>
  }
}
