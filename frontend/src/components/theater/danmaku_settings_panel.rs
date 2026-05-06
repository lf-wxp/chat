//! Danmaku settings panel (Req 12.5).
//!
//! Mirrors the subtitle settings panel but governs the danmaku
//! overlay instead — visibility, opacity, font-size tier, and
//! scroll-speed tier. Each change is persisted to localStorage
//! through [`TheaterState::persist_overlay_settings`].
//!
//! The panel does not render the composer itself — that widget lives
//! in [`DanmakuInput`](super::DanmakuInput) so the settings and the
//! text input can be placed independently by the parent layout.

use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlInputElement};

use crate::i18n;
use crate::theater::use_theater_state;

/// Danmaku settings panel.
#[component]
pub fn DanmakuSettingsPanel() -> impl IntoView {
  let state = use_theater_state();
  let i18n = i18n::use_i18n();

  let handle_visible_toggle = move |_| {
    state
      .overlay_settings
      .update(|s| s.danmaku_visible = !s.danmaku_visible);
    state.persist_overlay_settings();
  };

  let handle_opacity_input = move |ev: Event| {
    let raw = event_target_value(&ev);
    let parsed = raw.parse::<u8>().unwrap_or(100).min(100);
    state
      .overlay_settings
      .update(|s| s.danmaku_opacity = parsed);
    state.persist_overlay_settings();
  };

  let handle_font_size = move |ev: Event| {
    let Some(select) = ev
      .target()
      .and_then(|t| t.dyn_into::<HtmlInputElement>().ok())
    else {
      return;
    };
    let value = select.value();
    state
      .overlay_settings
      .update(|s| s.danmaku_font_size = value);
    state.persist_overlay_settings();
  };

  let handle_speed = move |ev: Event| {
    let Some(select) = ev
      .target()
      .and_then(|t| t.dyn_into::<HtmlInputElement>().ok())
    else {
      return;
    };
    let value = select.value();
    state.overlay_settings.update(|s| s.danmaku_speed = value);
    state.persist_overlay_settings();
  };

  view! {
    <section
      class="danmaku-settings"
      aria-label=move || t_string!(i18n, theater.danmaku_settings)
      data-testid="danmaku-settings-panel"
    >
      <header class="danmaku-settings__header">
        <h3 class="danmaku-settings__title">{t!(i18n, theater.danmaku_settings)}</h3>
      </header>

      <label class="danmaku-settings__row">
        <input
          type="checkbox"
          prop:checked=move || state.overlay_settings.with(|s| s.danmaku_visible)
          on:change=handle_visible_toggle
          data-testid="danmaku-visible-toggle"
        />
        <span>{t!(i18n, theater.danmaku_visible)}</span>
      </label>

      <label class="danmaku-settings__row">
        <span>{t!(i18n, theater.danmaku_opacity)}</span>
        <input
          type="range"
          min="0"
          max="100"
          step="5"
          prop:value=move || state.overlay_settings.with(|s| s.danmaku_opacity.to_string())
          on:input=handle_opacity_input
          data-testid="danmaku-opacity-slider"
        />
      </label>

      <fieldset class="danmaku-settings__group">
        <legend>{t!(i18n, theater.danmaku_font_size)}</legend>
        <select
          class="input"
          on:change=handle_font_size
          prop:value=move || state.overlay_settings.with(|s| s.danmaku_font_size.clone())
          data-testid="danmaku-font-size"
        >
          <option value="small">{t!(i18n, theater.subtitle_font_size_small)}</option>
          <option value="medium">{t!(i18n, theater.subtitle_font_size_medium)}</option>
          <option value="large">{t!(i18n, theater.subtitle_font_size_large)}</option>
        </select>
      </fieldset>

      <fieldset class="danmaku-settings__group">
        <legend>{t!(i18n, theater.danmaku_speed)}</legend>
        <select
          class="input"
          on:change=handle_speed
          prop:value=move || state.overlay_settings.with(|s| s.danmaku_speed.clone())
          data-testid="danmaku-speed"
        >
          <option value="slow">{t!(i18n, theater.danmaku_speed_slow)}</option>
          <option value="medium">{t!(i18n, theater.danmaku_speed_medium)}</option>
          <option value="fast">{t!(i18n, theater.danmaku_speed_fast)}</option>
        </select>
      </fieldset>
    </section>
  }
}
