//! Audio/video device & quality preferences.
//!
//! Enumerates `navigator.mediaDevices.enumerateDevices()` (lazily, via
//! `load_devices`) and lets the user pick a preferred default for
//! camera, microphone and speaker. Also exposes the speaker-volume
//! slider and the video-quality radio group.

use super::av_helpers::{DeviceCache, DeviceEntry, DeviceKind, enumerate_devices};
use super::camera_preview::CameraPreview;
use super::class_helpers::segmented_item_class;
use super::device_select::DeviceSelect;
use super::mic_level_feedback::MicrophoneLevelFeedback;
use super::permission_badge::{PermissionBadge, PermissionState};
use crate::i18n;
use crate::settings::{VideoQualityPref, use_settings_state};
use icondata as i;
use leptos::prelude::*;
use leptos_i18n::t;
use leptos_icons::Icon;

/// Audio & video section.
#[component]
pub fn AvSection() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let settings = use_settings_state();

  // Device catalogue — backed by a parent-scoped cache so the list
  // persists across drawer open/close cycles (V3-M-3). The loading
  // flags (`load_error`, `needs_permission`, `permission_denied`)
  // are also kept on the cache so the async `enumerate_devices`
  // task can safely write to them after `AvSection` has been
  // unmounted (fix for the `panic_already_borrowed` /
  // `Get::get` panics triggered when the user closed the drawer
  // mid-prompt).
  let window = leptos_use::use_window();
  let cache = expect_context::<DeviceCache>();
  let devices = cache.devices;
  let load_error = cache.load_error;
  let needs_permission = cache.needs_permission;
  let permission_denied = cache.permission_denied;

  // On first mount we enumerate devices WITHOUT requesting permission
  // so the settings page does not trigger a browser prompt
  // spontaneously. Subsequent mounts skip enumeration because the
  // cache is already populated (V3-M-3).
  let window_for_init = window.clone();
  Effect::new(move |_| {
    if !cache.initialised.get_untracked() {
      cache.initialised.set(true);
      enumerate_devices(
        window_for_init.clone(),
        devices,
        load_error,
        needs_permission,
        permission_denied,
        false,
      );
    }
  });

  let window_for_reload = window.clone();
  let reload = move |_| {
    enumerate_devices(
      window_for_reload.clone(),
      devices,
      load_error,
      needs_permission,
      permission_denied,
      false,
    );
  };

  let window_for_perm = StoredValue::new(window.clone());

  let video_quality = Memo::new(move |_| settings.get().video_quality);
  let speaker_volume = Memo::new(move |_| settings.get().speaker_volume);
  let microphone_volume = Memo::new(move |_| settings.get().microphone_volume);

  // Pre-computed per-kind device lists so the `<For>` in each
  // `DeviceSelect` does not re-filter the full catalogue on every
  // signal change (V3-Q-3).
  let camera_devices: Memo<Vec<DeviceEntry>> = Memo::new(move |_| {
    devices.with(|list| {
      list
        .iter()
        .filter(|d| d.kind == DeviceKind::Camera)
        .cloned()
        .collect()
    })
  });
  let microphone_devices: Memo<Vec<DeviceEntry>> = Memo::new(move |_| {
    devices.with(|list| {
      list
        .iter()
        .filter(|d| d.kind == DeviceKind::Microphone)
        .cloned()
        .collect()
    })
  });
  let speaker_devices: Memo<Vec<DeviceEntry>> = Memo::new(move |_| {
    devices.with(|list| {
      list
        .iter()
        .filter(|d| d.kind == DeviceKind::Speaker)
        .cloned()
        .collect()
    })
  });

  // Whether the current browser reports any audio-output sinks. Used
  // to hide the Speaker selector on runtimes that lack `setSinkId`
  // (Firefox, Safari < 17 — see S-5).
  let any_speaker_devices = Memo::new(move |_| !speaker_devices.get().is_empty());

  view! {
    <section class="settings-section" aria-labelledby="av-heading">
      <h2 id="av-heading" class="settings-section-title">
        <Icon icon=i::LuVideo attr:class="settings-section-icon" />
        {t!(i18n, settings.av_settings)}
      </h2>

      // Camera select
      <DeviceSelect
        kind=DeviceKind::Camera
        filtered_devices=camera_devices
        label=t!(i18n, settings.default_camera).into_any()
      />

      // Camera preview (Req 13.1.4 — device test mechanism).
      <CameraPreview />

      // Microphone select
      <DeviceSelect
        kind=DeviceKind::Microphone
        filtered_devices=microphone_devices
        label=t!(i18n, settings.default_microphone).into_any()
      />

      // Speaker select -- hidden on runtimes that do not enumerate any
      // `audiooutput` devices (Firefox, Safari <17).
      <Show when=move || any_speaker_devices.get()>
        <DeviceSelect
          kind=DeviceKind::Speaker
          filtered_devices=speaker_devices
          label=t!(i18n, settings.default_speaker).into_any()
        />
      </Show>

      <Show when=move || needs_permission.get()>
        <div class="settings-row">
          <p class="settings-hint">
            {t!(i18n, settings.default_device_permission_hint)}
            " "
            <PermissionBadge state=Signal::derive(|| PermissionState::Prompt) />
          </p>
          <button
            class="btn-primary settings-action"
            on:click=move |_| {
              enumerate_devices(
                window_for_perm.get_value(),
                devices,
                load_error,
                needs_permission,
                permission_denied,
                true,
              );
            }
            data-testid="av-request-permission"
          >
            <Icon icon=i::LuKey />
            <span>{t!(i18n, settings.request_device_permission)}</span>
          </button>
        </div>
      </Show>
      <Show when=move || permission_denied.get()>
        <div class="settings-row">
          <p class="settings-error">
            {t!(i18n, settings.device_permission_denied)}
            " "
            <PermissionBadge state=Signal::derive(|| PermissionState::Denied) />
          </p>
        </div>
      </Show>
      <Show when=move || load_error.get().is_some()>
        <p class="settings-error">{t!(i18n, settings.default_device_load_failed)}</p>
      </Show>

      <div class="settings-row">
        <button class="btn-ghost settings-reload" on:click=reload>
          <Icon icon=i::LuRefreshCw />
          <span>{t!(i18n, settings.av_reload_devices)}</span>
        </button>
      </div>

      // Speaker volume slider
      <div class="settings-row">
        <label class="settings-label" for="settings-speaker-volume">
          {t!(i18n, settings.speaker_volume)}
        </label>
        <div class="settings-slider-row">
          <input
            id="settings-speaker-volume"
            type="range"
            class="settings-slider"
            min="0"
            max="100"
            step="1"
            prop:value=move || (speaker_volume.get() * 100.0).round()
            on:input=move |ev| {
              if let Ok(parsed) = event_target_value(&ev).parse::<u32>() {
                let clamped = u32::min(parsed, 100);
                settings.update(|s| {
                  s.speaker_volume = clamped as f32 / 100.0;
                });
              }
            }
          />
          <span class="settings-slider-readout">
            {move || format!("{}%", (speaker_volume.get() * 100.0).round() as i32)}
          </span>
        </div>
      </div>

      // Microphone volume slider + real-time level feedback (Req 13.1.3).
      // The level meter uses the MicrophoneLevelFeedback component to
      // capture the default mic and render an animated bar via the
      // Web Audio API AnalyserNode.
      <div class="settings-row">
        <label class="settings-label" for="settings-microphone-volume">
          {t!(i18n, settings.microphone_volume)}
        </label>
        <div class="settings-slider-row">
          <input
            id="settings-microphone-volume"
            type="range"
            class="settings-slider"
            min="0"
            max="100"
            step="1"
            prop:value=move || (microphone_volume.get() * 100.0).round()
            on:input=move |ev| {
              if let Ok(parsed) = event_target_value(&ev).parse::<u32>() {
                let clamped = u32::min(parsed, 100);
                settings.update(|s| {
                  s.microphone_volume = clamped as f32 / 100.0;
                });
              }
            }
          />
          <span class="settings-slider-readout">
            {move || format!("{}%", (microphone_volume.get() * 100.0).round() as i32)}
          </span>
        </div>
      </div>

      // Real-time microphone level meter (Req 13.1.3).
      <MicrophoneLevelFeedback />

      // Video quality selector
      <div class="settings-row">
        <label class="settings-label">{t!(i18n, settings.video_quality)}</label>
        <div class="segmented" role="group" data-testid="video-quality-group">
          {[
            (VideoQualityPref::Auto, "video-quality-auto"),
            (VideoQualityPref::Low, "video-quality-low"),
            (VideoQualityPref::Standard, "video-quality-standard"),
            (VideoQualityPref::High, "video-quality-high"),
          ]
            .into_iter()
            .map(|(quality, testid)| {
              let label = match quality {
                VideoQualityPref::Auto => t!(i18n, settings.video_quality_auto).into_any(),
                VideoQualityPref::Low => t!(i18n, settings.video_quality_low).into_any(),
                VideoQualityPref::Standard => {
                  t!(i18n, settings.video_quality_standard).into_any()
                }
                VideoQualityPref::High => t!(i18n, settings.video_quality_high).into_any(),
              };
              view! {
                <button
                  class=move || segmented_item_class(video_quality.get() == quality)
                  on:click=move |_| {
                    settings.update(|s| s.video_quality = quality);
                  }
                  aria-pressed=move || (video_quality.get() == quality).to_string()
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
