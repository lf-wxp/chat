//! Audio/video device & quality preferences.
//!
//! Enumerates `navigator.mediaDevices.enumerateDevices()` (lazily, via
//! `load_devices`) and lets the user pick a preferred default for
//! camera, microphone and speaker. Also exposes the speaker-volume
//! slider and the video-quality radio group.

use super::class_helpers::segmented_item_class;
use super::device_select::DeviceSelect;
use super::permission_badge::{PermissionBadge, PermissionState};
use crate::i18n;
use crate::settings::{VideoQualityPref, use_settings_state};
use icondata as i;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_i18n::{t, t_string};
use leptos_icons::Icon;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{MediaDeviceInfo, MediaStreamConstraints};

/// One enumerated input/output device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeviceEntry {
  pub device_id: String,
  pub label: String,
  pub kind: DeviceKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DeviceKind {
  Camera,
  Microphone,
  Speaker,
}

/// Cached device list that outlives the `<Show when=open>` toggle so
/// `AvSection` does not re-enumerate devices on every drawer
/// open/close cycle (V3-M-3). Created in `SettingsPage` and provided
/// via `provide_context`.
#[derive(Clone)]
pub(super) struct DeviceCache {
  pub devices: RwSignal<Vec<DeviceEntry>>,
  /// Whether the initial enumeration has been performed at least once.
  pub initialised: RwSignal<bool>,
}

impl DeviceCache {
  pub fn new() -> Self {
    Self {
      devices: RwSignal::new(Vec::new()),
      initialised: RwSignal::new(false),
    }
  }
}

/// Audio & video section.
#[component]
pub fn AvSection() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let settings = use_settings_state();

  // Device catalogue — backed by a parent-scoped cache so the list
  // persists across drawer open/close cycles (V3-M-3). Only the
  // first mount triggers an automatic enumeration; subsequent mounts
  // reuse the cached list. The user can still force a refresh via
  // the "Reload" button.
  let window = leptos_use::use_window();
  let cache = expect_context::<DeviceCache>();
  let devices = cache.devices;
  let load_error: RwSignal<Option<String>> = RwSignal::new(None);
  let needs_permission: RwSignal<bool> = RwSignal::new(false);
  // Set to `true` when the user explicitly requested camera /
  // microphone access and the prompt was denied. Drives a dedicated
  // "permission denied" hint so the user understands they need to
  // re-enable access in the browser settings (V2-Q-2).
  let permission_denied: RwSignal<bool> = RwSignal::new(false);

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
            <PermissionBadge state=PermissionState::Prompt />
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
            <PermissionBadge state=PermissionState::Denied />
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

/// Enumerate devices via `navigator.mediaDevices.enumerateDevices()`.
///
/// When `prompt_permission` is `true`, first requests the user-media
/// permission so the subsequent enumeration returns populated device
/// labels. When `false`, the browser returns devices with empty
/// labels whenever permission has not yet been granted — the UI
/// surfaces a "Request Device Permission" button instead of silently
/// triggering a prompt.
fn enumerate_devices(
  window: leptos_use::UseWindow,
  devices: RwSignal<Vec<DeviceEntry>>,
  load_error: RwSignal<Option<String>>,
  needs_permission: RwSignal<bool>,
  permission_denied: RwSignal<bool>,
  prompt_permission: bool,
) {
  let Some(window) = window.as_ref().cloned() else {
    load_error.set(Some("no window".to_string()));
    return;
  };
  spawn_local(async move {
    let navigator = window.navigator();
    let media_devices = match navigator.media_devices() {
      Ok(m) => m,
      Err(_) => {
        load_error.set(Some("unavailable".to_string()));
        return;
      }
    };

    if prompt_permission {
      let constraints = MediaStreamConstraints::new();
      constraints.set_audio(&wasm_bindgen::JsValue::TRUE);
      constraints.set_video(&wasm_bindgen::JsValue::TRUE);
      if let Ok(promise) = media_devices.get_user_media_with_constraints(&constraints) {
        match JsFuture::from(promise).await {
          Ok(stream_val) => {
            needs_permission.set(false);
            permission_denied.set(false);
            if let Ok(stream) = stream_val.dyn_into::<web_sys::MediaStream>() {
              // Stop tracks immediately — we only wanted the labels.
              let tracks = stream.get_tracks();
              for i in 0..tracks.length() {
                if let Some(track) = tracks.get(i).dyn_ref::<web_sys::MediaStreamTrack>() {
                  track.stop();
                }
              }
            }
          }
          Err(_) => {
            needs_permission.set(true);
            permission_denied.set(true);
          }
        }
      }
    }

    let promise = match media_devices.enumerate_devices() {
      Ok(p) => p,
      Err(_) => {
        load_error.set(Some("enumerate_failed".to_string()));
        return;
      }
    };

    let list = match JsFuture::from(promise).await {
      Ok(list) => list,
      Err(_) => {
        load_error.set(Some("enumerate_rejected".to_string()));
        return;
      }
    };

    let array = js_sys::Array::from(&list);
    let mut parsed: Vec<DeviceEntry> = Vec::with_capacity(array.length() as usize);
    let mut any_unlabelled = false;
    for (idx, value) in array.iter().enumerate() {
      let Ok(device) = value.dyn_into::<MediaDeviceInfo>() else {
        continue;
      };
      let kind_str = js_sys::Reflect::get(&device, &wasm_bindgen::JsValue::from_str("kind"))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_default();
      let kind = match kind_str.as_str() {
        "videoinput" => DeviceKind::Camera,
        "audioinput" => DeviceKind::Microphone,
        "audiooutput" => DeviceKind::Speaker,
        _ => continue,
      };
      let raw_label = device.label();
      if raw_label.is_empty() {
        any_unlabelled = true;
      }
      let label = if raw_label.is_empty() {
        default_label(kind, idx)
      } else {
        raw_label
      };
      parsed.push(DeviceEntry {
        device_id: device.device_id(),
        label,
        kind,
      });
    }

    // If at least one device is reported but any of them lacks a
    // label, the user has not granted permission yet — surface the
    // "Request Device Permission" button rather than silently
    // showing "Camera 1".
    needs_permission.set(any_unlabelled && !parsed.is_empty());
    load_error.set(None);
    devices.set(parsed);
  });
}

fn default_label(kind: DeviceKind, idx: usize) -> String {
  let prefix = match kind {
    DeviceKind::Camera => "Camera",
    DeviceKind::Microphone => "Microphone",
    DeviceKind::Speaker => "Speaker",
  };
  format!("{prefix} {}", idx + 1)
}

// ---------------------------------------------------------------------------
// Unit tests — types that can be exercised without a browser runtime.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn device_kind_default_label_generates_expected_names() {
    assert_eq!(default_label(DeviceKind::Camera, 0), "Camera 1");
    assert_eq!(default_label(DeviceKind::Camera, 1), "Camera 2");
    assert_eq!(default_label(DeviceKind::Microphone, 0), "Microphone 1");
    assert_eq!(default_label(DeviceKind::Speaker, 2), "Speaker 3");
  }

  #[test]
  fn device_entry_equality_and_clone() {
    let a = DeviceEntry {
      device_id: "id-1".into(),
      label: "Label A".into(),
      kind: DeviceKind::Camera,
    };
    let b = DeviceEntry {
      device_id: "id-1".into(),
      label: "Label A".into(),
      kind: DeviceKind::Camera,
    };
    let c = DeviceEntry {
      device_id: "id-2".into(),
      label: "Label B".into(),
      kind: DeviceKind::Microphone,
    };
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_eq!(a.clone(), a);
  }

  #[test]
  fn device_kind_debug_and_clone() {
    let kind = DeviceKind::Microphone;
    let dbg = format!("{:?}", kind);
    assert!(!dbg.is_empty());
    assert_eq!(kind, kind.clone());
  }
}

/// Real-time microphone level feedback (Req 13.1.3).
///
/// Captures the default microphone via `getUserMedia`, pipes the
/// stream through a Web Audio `AnalyserNode`, and reads the RMS
/// (root-mean-square) volume on a `requestAnimationFrame` loop
/// powered by `leptos_use::use_raf_fn`. The level is displayed as
/// an animated bar. The bar is hidden when the component unmounts
/// or when microphone access is denied.
#[component]
fn MicrophoneLevelFeedback() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let level: RwSignal<f32> = RwSignal::new(0.0);
  let has_access: RwSignal<bool> = RwSignal::new(false);
  let access_denied: RwSignal<bool> = RwSignal::new(false);

  // Shared analyser node reference — populated once the audio graph
  // is built, then read on every animation frame by `use_raf_fn`.
  let analyser: StoredValue<Option<wasm_bindgen::JsValue>> = StoredValue::new(None);
  // Keep a handle to the stream so we can stop tracks on cleanup.
  let stream_handle: StoredValue<Option<web_sys::MediaStream>> = StoredValue::new(None);
  // Keep a handle to the audio context so we can close it on cleanup.
  let audio_ctx_handle: StoredValue<Option<wasm_bindgen::JsValue>> = StoredValue::new(None);

  // On mount, request microphone access and build the audio graph.
  Effect::new(move |_| {
    spawn_local(async move {
      let window = leptos_use::use_window();
      let Some(window) = window.as_ref() else {
        return;
      };
      let navigator = window.navigator();
      let media_devices = match navigator.media_devices() {
        Ok(m) => m,
        Err(_) => return,
      };
      let constraints = MediaStreamConstraints::new();
      constraints.set_audio(&wasm_bindgen::JsValue::TRUE);
      let Ok(promise) = media_devices.get_user_media_with_constraints(&constraints) else {
        access_denied.set(true);
        return;
      };
      let Ok(stream_val) = wasm_bindgen_futures::JsFuture::from(promise).await else {
        access_denied.set(true);
        return;
      };
      let Ok(stream) = stream_val.dyn_into::<web_sys::MediaStream>() else {
        access_denied.set(true);
        return;
      };

      // Build Web Audio graph: source → analyser.
      let audio_ctx_ctor =
        js_sys::Reflect::get(window, &wasm_bindgen::JsValue::from_str("AudioContext"))
          .ok()
          .and_then(|v| v.dyn_into::<js_sys::Function>().ok());
      let Some(audio_ctx_ctor) = audio_ctx_ctor else {
        access_denied.set(true);
        return;
      };
      let Ok(audio_ctx_val) = js_sys::Reflect::construct(&audio_ctx_ctor, &js_sys::Array::new())
      else {
        access_denied.set(true);
        return;
      };
      let create_analyser = js_sys::Reflect::get(
        &audio_ctx_val,
        &wasm_bindgen::JsValue::from_str("createAnalyser"),
      )
      .ok()
      .and_then(|v| v.dyn_into::<js_sys::Function>().ok());
      let Some(create_analyser) = create_analyser else {
        access_denied.set(true);
        return;
      };
      let Ok(analyser_val) = create_analyser.call0(&audio_ctx_val) else {
        access_denied.set(true);
        return;
      };

      // Configure analyser: fftSize = 256 → 128 frequency bins.
      let _ = js_sys::Reflect::set(
        &analyser_val,
        &wasm_bindgen::JsValue::from_str("fftSize"),
        &wasm_bindgen::JsValue::from_f64(256.0),
      );
      // SmoothingTimeConstant for less jittery meter.
      let _ = js_sys::Reflect::set(
        &analyser_val,
        &wasm_bindgen::JsValue::from_str("smoothingTimeConstant"),
        &wasm_bindgen::JsValue::from_f64(0.8),
      );

      // Create MediaStreamSource from the microphone stream.
      let create_source = js_sys::Reflect::get(
        &audio_ctx_val,
        &wasm_bindgen::JsValue::from_str("createMediaStreamSource"),
      )
      .ok()
      .and_then(|v| v.dyn_into::<js_sys::Function>().ok());
      let Some(create_source) = create_source else {
        access_denied.set(true);
        return;
      };
      let Ok(source_val) = create_source.call1(&audio_ctx_val, &stream) else {
        access_denied.set(true);
        return;
      };

      // Connect: source → analyser.
      let connect_fn =
        js_sys::Reflect::get(&source_val, &wasm_bindgen::JsValue::from_str("connect"))
          .ok()
          .and_then(|v| v.dyn_into::<js_sys::Function>().ok());
      if let Some(connect_fn) = connect_fn {
        let _ = connect_fn.call1(&source_val, &analyser_val);
      }

      // Store references for the rAF callback and cleanup.
      analyser.set_value(Some(analyser_val));
      stream_handle.set_value(Some(stream));
      audio_ctx_handle.set_value(Some(audio_ctx_val));
      has_access.set(true);
    });
  });

  // Animation loop powered by `leptos_use::use_raf_fn`. This
  // replaces the manual `requestAnimationFrame` + `Rc<RefCell<
  // Option<Closure>>>` pattern. `use_raf_fn` automatically handles
  // cleanup when the component unmounts.
  let _raf = leptos_use::use_raf_fn_with_options(
    move |_args| {
      analyser.with_value(|opt| {
        let Some(analyser_val) = opt.as_ref() else {
          return;
        };
        let buffer = js_sys::Uint8Array::new_with_length(128);
        let get_byte_freq = js_sys::Reflect::get(
          analyser_val,
          &wasm_bindgen::JsValue::from_str("getByteFrequencyData"),
        )
        .ok()
        .and_then(|v| v.dyn_into::<js_sys::Function>().ok());
        if let Some(get_byte_freq) = get_byte_freq {
          let _ = get_byte_freq.call1(analyser_val, &buffer);
          let data = buffer.to_vec();
          let sum: f32 = data.iter().map(|&v| (v as f32 / 255.0).powi(2)).sum();
          let rms = (sum / data.len() as f32).sqrt();
          level.set(rms);
        }
      });
    },
    leptos_use::UseRafFnOptions::default(),
  );

  // Cleanup: close the audio context and stop mic tracks when the
  // component unmounts. `use_raf_fn` already cancels the animation
  // loop, but we still need to release the microphone and audio
  // context resources.
  on_cleanup(move || {
    // Close the audio context.
    audio_ctx_handle.with_value(|opt| {
      if let Some(ctx) = opt.as_ref() {
        let close_fn = js_sys::Reflect::get(ctx, &wasm_bindgen::JsValue::from_str("close"))
          .ok()
          .and_then(|v| v.dyn_into::<js_sys::Function>().ok());
        if let Some(close_fn) = close_fn {
          let _ = close_fn.call0(ctx);
        }
      }
    });
    // Stop all microphone tracks.
    stream_handle.with_value(|opt| {
      if let Some(stream) = opt.as_ref() {
        let tracks = stream.get_tracks();
        for i in 0..tracks.length() {
          if let Some(track) = tracks.get(i).dyn_ref::<web_sys::MediaStreamTrack>() {
            track.stop();
          }
        }
      }
    });
  });

  view! {
    <Show when=move || has_access.get()>
      <div class="settings-row settings-mic-level-row">
        <label class="settings-label">{t!(i18n, settings.microphone_level)}</label>
        <div class="settings-mic-level-meter" role="meter"
          aria-label=move || t_string!(i18n, settings.microphone_level)
          aria-valuemin="0"
          aria-valuemax="100"
          aria-valuenow=move || (level.get() * 100.0).round() as i32
        >
          <div
            class="settings-mic-level-bar"
            style:max-width=move || format!("{}%", (level.get() * 100.0).round() as i32)
          ></div>
        </div>
      </div>
    </Show>
    <Show when=move || access_denied.get()>
      <p class="settings-hint">{t!(i18n, settings.microphone_level_denied)}</p>
    </Show>
  }
}
