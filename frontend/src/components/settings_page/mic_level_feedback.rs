//! Real-time microphone level feedback component (Req 13.1.3).
//!
//! Requires the user to explicitly click a "Test Microphone" button
//! before starting the audio graph. This avoids unnecessary microphone
//! access prompts and prevents the mic from being active without user
//! intent (O-1).
//!
//! Captures the default microphone via `getUserMedia`, pipes the stream
//! through a typed `web_sys::AnalyserNode`, and reads the RMS volume on
//! a `requestAnimationFrame` loop powered by `leptos_use::use_raf_fn`.
//!
//! Uses typed `web_sys` bindings instead of `js_sys::Reflect` for
//! compile-time type safety (A11).
//!
//! If the stream is acquired but a subsequent step fails, the
//! microphone tracks are stopped immediately (B-1).

use crate::i18n;
use icondata as i;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_i18n::{t, t_string};
use leptos_icons::Icon;
use wasm_bindgen::JsCast;
use web_sys::{AnalyserNode, AudioContext, MediaStream, MediaStreamConstraints};

/// Real-time microphone level feedback (Req 13.1.3).
#[component]
pub(super) fn MicrophoneLevelFeedback() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let level: RwSignal<f32> = RwSignal::new(0.0);
  let has_access: RwSignal<bool> = RwSignal::new(false);
  let access_denied: RwSignal<bool> = RwSignal::new(false);
  // Whether the user has clicked the "Test Microphone" button.
  let testing: RwSignal<bool> = RwSignal::new(false);

  // Typed analyser reference for the rAF loop.
  let analyser: StoredValue<Option<AnalyserNode>> = StoredValue::new(None);
  // Keep a handle to the stream so we can stop tracks on cleanup.
  let stream_handle: StoredValue<Option<MediaStream>> = StoredValue::new(None);
  // Keep a handle to the audio context so we can close it on cleanup.
  let audio_ctx_handle: StoredValue<Option<AudioContext>> = StoredValue::new(None);

  /// Stop all tracks on the given stream.
  fn stop_stream_tracks(stream: &MediaStream) {
    let tracks = stream.get_tracks();
    for i in 0..tracks.length() {
      if let Some(track) = tracks.get(i).dyn_ref::<web_sys::MediaStreamTrack>() {
        track.stop();
      }
    }
  }

  /// Start microphone capture and build the Web Audio graph using
  /// typed `web_sys` bindings (A11).
  fn start_mic_capture(
    has_access: RwSignal<bool>,
    access_denied: RwSignal<bool>,
    analyser: StoredValue<Option<AnalyserNode>>,
    stream_handle: StoredValue<Option<MediaStream>>,
    audio_ctx_handle: StoredValue<Option<AudioContext>>,
  ) {
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
      let Ok(stream) = stream_val.dyn_into::<MediaStream>() else {
        access_denied.set(true);
        return;
      };

      // Build Web Audio graph using typed API.
      let ctx = match AudioContext::new() {
        Ok(ctx) => ctx,
        Err(_) => {
          stop_stream_tracks(&stream);
          access_denied.set(true);
          return;
        }
      };

      let analyser_node = match ctx.create_analyser() {
        Ok(a) => a,
        Err(_) => {
          stop_stream_tracks(&stream);
          access_denied.set(true);
          return;
        }
      };
      analyser_node.set_fft_size(256);
      analyser_node.set_smoothing_time_constant(0.8);

      let source = match ctx.create_media_stream_source(&stream) {
        Ok(s) => s,
        Err(_) => {
          stop_stream_tracks(&stream);
          access_denied.set(true);
          return;
        }
      };

      // Connect: source → analyser.
      if source.connect_with_audio_node(&analyser_node).is_err() {
        stop_stream_tracks(&stream);
        access_denied.set(true);
        return;
      }

      // Store references for the rAF callback and cleanup.
      analyser.set_value(Some(analyser_node));
      stream_handle.set_value(Some(stream));
      audio_ctx_handle.set_value(Some(ctx));
      has_access.set(true);
    });
  }

  /// Stop microphone capture and clean up resources.
  fn stop_mic_capture(
    level: RwSignal<f32>,
    has_access: RwSignal<bool>,
    testing: RwSignal<bool>,
    analyser: StoredValue<Option<AnalyserNode>>,
    stream_handle: StoredValue<Option<MediaStream>>,
    audio_ctx_handle: StoredValue<Option<AudioContext>>,
  ) {
    // Close the audio context.
    audio_ctx_handle.with_value(|opt| {
      if let Some(ctx) = opt.as_ref() {
        let _ = ctx.close();
      }
    });
    // Stop all microphone tracks.
    stream_handle.with_value(|opt| {
      if let Some(stream) = opt.as_ref() {
        stop_stream_tracks(stream);
      }
    });
    // Reset state.
    analyser.set_value(None);
    stream_handle.set_value(None);
    audio_ctx_handle.set_value(None);
    has_access.set(false);
    level.set(0.0);
    testing.set(false);
  }

  // Animation loop — reads frequency data from the typed AnalyserNode.
  let _raf = leptos_use::use_raf_fn_with_options(
    move |_args| {
      analyser.with_value(|opt| {
        let Some(analyser_node) = opt.as_ref() else {
          return;
        };
        let mut buffer = vec![0u8; analyser_node.frequency_bin_count() as usize];
        analyser_node.get_byte_frequency_data(&mut buffer);
        let sum: f32 = buffer.iter().map(|&v| (v as f32 / 255.0).powi(2)).sum();
        let rms = (sum / buffer.len() as f32).sqrt();
        level.set(rms);
      });
    },
    leptos_use::UseRafFnOptions::default(),
  );

  // Cleanup on component unmount — stop mic if currently active.
  on_cleanup(move || {
    if testing.get_untracked() {
      stop_mic_capture(
        level,
        has_access,
        testing,
        analyser,
        stream_handle,
        audio_ctx_handle,
      );
    }
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
        <button
          class="btn-ghost settings-action"
          on:click=move |_| {
            stop_mic_capture(
              level,
              has_access,
              testing,
              analyser,
              stream_handle,
              audio_ctx_handle,
            );
          }
          data-testid="stop-test-microphone"
        >
          <Icon icon=i::LuMicOff />
          <span>{t!(i18n, settings.stop_test_microphone)}</span>
        </button>
      </div>
    </Show>
    <Show when=move || !has_access.get() && !access_denied.get()>
      <div class="settings-row">
        <button
          class="btn-primary settings-action"
          on:click=move |_| {
            testing.set(true);
            start_mic_capture(
              has_access,
              access_denied,
              analyser,
              stream_handle,
              audio_ctx_handle,
            );
          }
          data-testid="test-microphone"
        >
          <Icon icon=i::LuMic />
          <span>{t!(i18n, settings.test_microphone)}</span>
        </button>
      </div>
    </Show>
    <Show when=move || access_denied.get()>
      <p class="settings-hint">{t!(i18n, settings.microphone_level_denied)}</p>
    </Show>
  }
}
