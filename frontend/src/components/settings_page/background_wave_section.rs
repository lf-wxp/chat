//! Gradient Waves shader tuning sliders (scale, ratio, speed, swell,
//! turbulence, tilt, zoom, horizon height, fog depth, brightness,
//! opacity).
//!
//! Split out of `background_section.rs` because eleven independent
//! sliders — each needing its own id/label/handler — would have
//! pushed that file well past a comfortable size. Rendered inline
//! inside `BackgroundSection`'s settings list (not a nested
//! `<section>`) so it reads as a continuation of the same
//! "Background" settings group, only visible while the Gradient
//! Waves effect can actually be seen.

use super::background_section_helpers::{
  slider_percent_to_wave_brightness, slider_percent_to_wave_fog_depth,
  slider_percent_to_wave_horizon_height, slider_percent_to_wave_opacity,
  slider_percent_to_wave_ratio, slider_percent_to_wave_scale, slider_percent_to_wave_speed,
  slider_percent_to_wave_swell, slider_percent_to_wave_tilt, slider_percent_to_wave_turbulence,
  slider_percent_to_wave_zoom, wave_brightness_to_slider_percent, wave_fog_depth_to_slider_percent,
  wave_horizon_height_to_slider_percent, wave_opacity_to_slider_percent,
  wave_ratio_to_slider_percent, wave_scale_to_slider_percent, wave_speed_to_slider_percent,
  wave_swell_to_slider_percent, wave_tilt_to_slider_percent, wave_turbulence_to_slider_percent,
  wave_zoom_to_slider_percent,
};
use crate::i18n;
use crate::settings::{BackgroundEffects, use_settings_state};
use icondata as i;
use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use leptos_icons::Icon;
use wasm_bindgen::JsCast;

/// Expands to a single settings-row slider bound to one
/// `WaveConfig` field. Declared as a macro (rather than a shared
/// sub-component/helper struct) because each row's i18n key must be
/// a literal path for `t!`/`t_string!` to resolve at macro-expansion
/// time — a function-pointer table can't carry that through.
///
/// * `$field` — the `WaveConfig` field this row reads/writes.
/// * `$to_value` / `$to_percent` — the slider ↔ value converters
///   from `background_section_helpers`.
/// * `$icon` — `icondata` icon constant.
/// * `$key` — the `settings.background_wave_*` i18n key (bare path,
///   no quotes — matches `t!`'s own calling convention).
/// * `$id` — HTML `id`/`data-testid` string for this row.
macro_rules! wave_slider_row {
  ($i18n:expr, $settings:expr, $field:ident, $to_value:expr, $to_percent:expr, $icon:expr, $key:ident, $id:expr) => {{
    let i18n = $i18n;
    let settings = $settings;
    let percent = Memo::new(move |_| $to_percent(settings.get().background.waves.$field));
    let on_input = move |ev: leptos::ev::Event| {
      let Some(target) = ev.target() else { return };
      let Ok(input) = target.dyn_into::<web_sys::HtmlInputElement>() else {
        return;
      };
      let Ok(pct) = input.value().parse::<u8>() else {
        return;
      };
      let value = $to_value(pct);
      settings.update(|s| s.background.waves.$field = value);
    };

    view! {
      <div class="settings-row">
        <label class="settings-label" for=$id>
          <Icon icon=$icon attr:class="settings-label-icon" />
          {t!(i18n, settings.$key)}
        </label>
        <div class="bg-section__slider-wrapper">
          <input
            id=$id
            type="range"
            min="0"
            max="100"
            step="1"
            class="bg-section__slider"
            prop:value=move || percent.get().to_string()
            on:input=on_input
            aria-label=move || t_string!(i18n, settings.$key)
            data-testid=$id
          />
          <span class="bg-section__slider-value" aria-hidden="true">
            {move || format!("{}%", percent.get())}
          </span>
        </div>
      </div>
    }
  }};
}

/// Gradient Waves tuning panel. Renders eleven sliders, each bound
/// straight to a `WaveConfig` field. Hidden entirely when the
/// current `BackgroundEffects` selection can't possibly render the
/// waves shader (Rays/Particles/None only).
#[component]
pub fn BackgroundWaveSection() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let settings = use_settings_state();

  let effects = Memo::new(move |_| settings.get().background.effects);
  let waves_active = Memo::new(move |_| {
    matches!(
      effects.get(),
      BackgroundEffects::All | BackgroundEffects::Waves
    )
  });

  view! {
    <Show when=move || waves_active.get()>
      {wave_slider_row!(
        i18n, settings, scale,
        slider_percent_to_wave_scale, wave_scale_to_slider_percent,
        i::LuScale, background_wave_scale, "bg-wave-scale-slider"
      )}
      {wave_slider_row!(
        i18n, settings, ratio,
        slider_percent_to_wave_ratio, wave_ratio_to_slider_percent,
        i::LuRatio, background_wave_ratio, "bg-wave-ratio-slider"
      )}
      {wave_slider_row!(
        i18n, settings, speed,
        slider_percent_to_wave_speed, wave_speed_to_slider_percent,
        i::LuGauge, background_wave_speed, "bg-wave-speed-slider"
      )}
      {wave_slider_row!(
        i18n, settings, swell,
        slider_percent_to_wave_swell, wave_swell_to_slider_percent,
        i::LuWaves, background_wave_swell, "bg-wave-swell-slider"
      )}
      {wave_slider_row!(
        i18n, settings, turbulence,
        slider_percent_to_wave_turbulence, wave_turbulence_to_slider_percent,
        i::LuWind, background_wave_turbulence, "bg-wave-turbulence-slider"
      )}
      {wave_slider_row!(
        i18n, settings, tilt,
        slider_percent_to_wave_tilt, wave_tilt_to_slider_percent,
        i::LuRotate3d, background_wave_tilt, "bg-wave-tilt-slider"
      )}
      {wave_slider_row!(
        i18n, settings, zoom,
        slider_percent_to_wave_zoom, wave_zoom_to_slider_percent,
        i::LuZoomIn, background_wave_zoom, "bg-wave-zoom-slider"
      )}
      {wave_slider_row!(
        i18n, settings, horizon_height,
        slider_percent_to_wave_horizon_height, wave_horizon_height_to_slider_percent,
        i::LuMountain, background_wave_horizon_height, "bg-wave-horizon-height-slider"
      )}
      {wave_slider_row!(
        i18n, settings, fog_depth,
        slider_percent_to_wave_fog_depth, wave_fog_depth_to_slider_percent,
        i::LuCloudFog, background_wave_fog_depth, "bg-wave-fog-depth-slider"
      )}
      {wave_slider_row!(
        i18n, settings, brightness,
        slider_percent_to_wave_brightness, wave_brightness_to_slider_percent,
        i::LuSun, background_wave_brightness, "bg-wave-brightness-slider"
      )}
      {wave_slider_row!(
        i18n, settings, opacity,
        slider_percent_to_wave_opacity, wave_opacity_to_slider_percent,
        i::LuContrast, background_wave_opacity, "bg-wave-opacity-slider"
      )}
    </Show>
  }
}
