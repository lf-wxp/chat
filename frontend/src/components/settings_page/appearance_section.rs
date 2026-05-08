//! Appearance settings (theme / language / font size).
//!
//! Consumers of this component do not need any props: it binds
//! directly to the `AppState` theme / locale signals and the
//! settings-store font scale.

use super::class_helpers::{segmented_item_class, toggle_root_class};
use crate::i18n;
use crate::settings::{FontScale, use_settings_state};
use crate::state::{self, use_app_state};
use icondata as i;
use leptos::prelude::*;
use leptos_i18n::{t, t_string};
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
  // Visual-effect toggles (V6 batch 8b). Mirrored to the <html> root
  // via an effect in `app.rs` so the CSS effects layer can react.
  let glass_enabled = Memo::new(move |_| settings.get().glass_enabled);
  let motion_enabled = Memo::new(move |_| settings.get().motion_enabled);

  let toggle_glass = move |_| {
    settings.update(|s| s.glass_enabled = !s.glass_enabled);
  };
  let toggle_motion = move |_| {
    settings.update(|s| s.motion_enabled = !s.motion_enabled);
  };

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
      class=move || segmented_item_class(theme.get() == state::THEME_LIGHT)
      on:click=move |_| theme.set(state::THEME_LIGHT.to_string())
      aria-pressed=move || (theme.get() == state::THEME_LIGHT).to_string()
          >
            <Icon icon=i::LuSun />
            <span>{t!(i18n, settings.theme_light)}</span>
          </button>
          <button
      class=move || segmented_item_class(theme.get() == state::THEME_DARK)
      on:click=move |_| theme.set(state::THEME_DARK.to_string())
      aria-pressed=move || (theme.get() == state::THEME_DARK).to_string()
          >
            <Icon icon=i::LuMoon />
            <span>{t!(i18n, settings.theme_dark)}</span>
          </button>
          <button
      class=move || segmented_item_class(theme.get() == state::THEME_SYSTEM)
      on:click=move |_| theme.set(state::THEME_SYSTEM.to_string())
      aria-pressed=move || (theme.get() == state::THEME_SYSTEM).to_string()
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
            class=move || segmented_item_class(locale.get() == state::LOCALE_EN)
            on:click=move |_| locale.set(state::LOCALE_EN.to_string())
            aria-pressed=move || (locale.get() == state::LOCALE_EN).to_string()
          >
            <span>"English"</span>
          </button>
          <button
            class=move || segmented_item_class(locale.get() == state::LOCALE_ZH_CN)
            on:click=move |_| locale.set(state::LOCALE_ZH_CN.to_string())
            aria-pressed=move || (locale.get() == state::LOCALE_ZH_CN).to_string()
          >
            <span>"中文"</span>
          </button>
          <button
            class=move || segmented_item_class(locale.get() == state::LOCALE_ES)
            on:click=move |_| locale.set(state::LOCALE_ES.to_string())
            aria-pressed=move || (locale.get() == state::LOCALE_ES).to_string()
          >
            <span>"Español"</span>
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

      // Visual effects (V6 batch 8b). Two boolean toggles mirrored to
      // the root <html> via effects in `app.rs`:
      //   * `data-glass`  → .glass-* CSS utilities + component bridges
      //   * `data-motion` → animations.css opt-out selector
      <div class="settings-row settings-toggle-row">
        <div class="settings-toggle-meta">
          <label class="settings-label">
            <Icon icon=i::LuSparkles attr:class="settings-label-icon" />
            {t!(i18n, settings.enable_glass)}
          </label>
          <p class="settings-hint">{t!(i18n, settings.enable_glass_hint)}</p>
        </div>
        <button
          class=move || toggle_root_class(glass_enabled.get())
          role="switch"
          aria-label=move || t_string!(i18n, settings.enable_glass)
          aria-checked=move || glass_enabled.get().to_string()
          on:click=toggle_glass
          data-testid="toggle-glass-enabled"
        >
          <span class="settings-toggle-thumb"></span>
        </button>
      </div>

      <div class="settings-row settings-toggle-row">
        <div class="settings-toggle-meta">
          <label class="settings-label">
            <Icon icon=i::LuWand attr:class="settings-label-icon" />
            {t!(i18n, settings.enable_motion)}
          </label>
          <p class="settings-hint">{t!(i18n, settings.enable_motion_hint)}</p>
        </div>
        <button
          class=move || toggle_root_class(motion_enabled.get())
          role="switch"
          aria-label=move || t_string!(i18n, settings.enable_motion)
          aria-checked=move || motion_enabled.get().to_string()
          on:click=toggle_motion
          data-testid="toggle-motion-enabled"
        >
          <span class="settings-toggle-thumb"></span>
        </button>
      </div>
    </section>
  }
}
