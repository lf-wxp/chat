//! Voice message bubble component with playback support
//!
//! Renders a voice message with play/pause button, real-time waveform visualization
//! driven by Web Audio API AnalyserNode, duration display, and progress indicator.

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;
use leptos_i18n::t_string;
use wasm_bindgen::JsCast;

use crate::i18n::*;

/// Number of waveform bars in the playback bubble
const PLAYBACK_BAR_COUNT: usize = 8;
/// Maximum bar height in pixels
const BAR_MAX_HEIGHT: f32 = 22.0;
/// Minimum bar height in pixels (idle state)
const BAR_MIN_HEIGHT: f32 = 4.0;

/// Default (idle) bar heights — varied for visual interest
const IDLE_HEIGHTS: [f32; PLAYBACK_BAR_COUNT] = [8.0, 14.0, 20.0, 12.0, 18.0, 10.0, 16.0, 12.0];

/// Playback audio analysis state for cleanup
struct PlaybackAnalysis {
  audio_ctx: web_sys::AudioContext,
  raf_id: Rc<RefCell<i32>>,
}

impl PlaybackAnalysis {
  fn cleanup(&self) {
    let id = *self.raf_id.borrow();
    if id != 0 {
      web_sys::window()
        .unwrap()
        .cancel_animation_frame(id)
        .ok();
      *self.raf_id.borrow_mut() = 0;
    }
    let _ = self.audio_ctx.close();
  }
}

/// Start the rAF loop to sample playback audio levels
fn start_playback_level_monitor(
  analyser: &web_sys::AnalyserNode,
  bar_heights: RwSignal<[f32; PLAYBACK_BAR_COUNT]>,
  raf_id: &Rc<RefCell<i32>>,
) {
  let analyser = analyser.clone();
  let raf_id = raf_id.clone();

  analyser.set_fft_size(256);
  analyser.set_smoothing_time_constant(0.65);

  let bin_count = analyser.frequency_bin_count() as usize;
  let buf = Rc::new(RefCell::new(vec![0u8; bin_count]));

  let closure: Rc<RefCell<Option<wasm_bindgen::closure::Closure<dyn Fn()>>>> =
    Rc::new(RefCell::new(None));
  let closure_clone = closure.clone();

  // Clone raf_id for the closure; the original stays available after
  let raf_id_inner = raf_id.clone();
  let cb = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
    let mut data = buf.borrow_mut();
    analyser.get_byte_frequency_data(&mut data);

    let band_size = data.len() / PLAYBACK_BAR_COUNT;
    let mut heights = [0.0f32; PLAYBACK_BAR_COUNT];
    for (i, h) in heights.iter_mut().enumerate() {
      let start = i * band_size;
      let end = (start + band_size).min(data.len());
      let sum: u32 = data[start..end].iter().map(|&v| v as u32).sum();
      let avg = sum as f32 / (end - start) as f32;
      let level = (avg / 255.0).max(0.08);
      *h = BAR_MIN_HEIGHT + level * (BAR_MAX_HEIGHT - BAR_MIN_HEIGHT);
    }
    bar_heights.set(heights);

    if let Some(ref c) = *closure_clone.borrow() {
      let id = web_sys::window()
        .unwrap()
        .request_animation_frame(c.as_ref().unchecked_ref())
        .unwrap_or(0);
      *raf_id_inner.borrow_mut() = id;
    }
  });

  let id = web_sys::window()
    .unwrap()
    .request_animation_frame(cb.as_ref().unchecked_ref())
    .unwrap_or(0);
  *raf_id.borrow_mut() = id;

  *closure.borrow_mut() = Some(cb);
  std::mem::forget(closure);
}

/// Voice message bubble with playback controls and real-time waveform
#[component]
pub(super) fn VoiceBubble(
  /// Raw audio data (webm/opus encoded)
  data: Vec<u8>,
  /// Duration in milliseconds
  duration_ms: u32,
) -> impl IntoView {
  let i18n = use_i18n();
  let duration_sec = duration_ms as f64 / 1000.0;

  // Playback state signals
  let is_playing = RwSignal::new(false);
  let playback_progress = RwSignal::new(0.0f64); // 0.0 to 1.0
  let bar_heights = RwSignal::new(IDLE_HEIGHTS);

  // Store audio element and analysis state for cleanup (Rc<RefCell> since WASM is single-threaded)
  let audio_element: RwSignal<Option<web_sys::HtmlAudioElement>> = RwSignal::new(None);
  let analysis_state: Rc<RefCell<Option<PlaybackAnalysis>>> = Rc::new(RefCell::new(None));

  let data_clone = data.clone();
  let analysis_pause = analysis_state.clone();
  let analysis_play_resume = analysis_state.clone();
  let analysis_play_new = analysis_state.clone();
  let analysis_ended = analysis_state.clone();

  let handle_play_pause = move |_: web_sys::MouseEvent| {
    if is_playing.get_untracked() {
      // Pause — stop analysis, keep audio element
      if let Some(audio) = audio_element.get_untracked() {
        let _ = audio.pause();
      }
      if let Some(ref state) = *analysis_pause.borrow() {
        state.cleanup();
      }
      *analysis_pause.borrow_mut() = None;
      bar_heights.set(IDLE_HEIGHTS);
      is_playing.set(false);
      return;
    }

    // Play
    if let Some(audio) = audio_element.get_untracked() {
      // Resume existing audio — re-create analysis
      let audio_ctx = web_sys::AudioContext::new().unwrap();
      let analyser = audio_ctx.create_analyser().unwrap();
      let source = audio_ctx
        .create_media_element_source(&audio)
        .unwrap();
      let _ = source.connect_with_audio_node(&analyser);
      let _ = analyser.connect_with_audio_node(&audio_ctx.destination());

      let raf_id = Rc::new(RefCell::new(0i32));
      start_playback_level_monitor(&analyser, bar_heights, &raf_id);

      *analysis_play_resume.borrow_mut() = Some(PlaybackAnalysis {
        audio_ctx,
        raf_id,
      });

      let _ = audio.play().unwrap();
      is_playing.set(true);
    } else {
      // Create new audio element from blob data
      let data_for_play = data_clone.clone();
      let array = js_sys::Uint8Array::new_with_length(data_for_play.len() as u32);
      array.copy_from(&data_for_play);

      let parts = js_sys::Array::new();
      parts.push(&array.buffer());

      let options = web_sys::BlobPropertyBag::new();
      options.set_type("audio/webm;codecs=opus");
      let blob =
        web_sys::Blob::new_with_buffer_source_sequence_and_options(&parts, &options).unwrap();

      let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();
      let audio = web_sys::HtmlAudioElement::new_with_src(&url).unwrap();

      // ── Web Audio API: AudioContext → AnalyserNode → destination ──
      let audio_ctx = web_sys::AudioContext::new().unwrap();
      let analyser = audio_ctx.create_analyser().unwrap();
      let source = audio_ctx
        .create_media_element_source(&audio)
        .unwrap();
      // source → analyser → destination (must connect to destination for audible output)
      let _ = source.connect_with_audio_node(&analyser);
      let _ = analyser.connect_with_audio_node(&audio_ctx.destination());

      let raf_id = Rc::new(RefCell::new(0i32));
      start_playback_level_monitor(&analyser, bar_heights, &raf_id);

      *analysis_play_new.borrow_mut() = Some(PlaybackAnalysis {
        audio_ctx,
        raf_id,
      });

      // Progress update via timeupdate event
      let audio_progress = audio.clone();
      let on_timeupdate = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
        let current = audio_progress.current_time();
        let total = audio_progress.duration();
        if total.is_finite() && total > 0.0 {
          playback_progress.set(current / total);
        }
      });
      audio.set_ontimeupdate(Some(on_timeupdate.as_ref().unchecked_ref()));
      on_timeupdate.forget();

      // Handle playback end
      let url_cleanup = url.clone();
      let analysis_end_ref = analysis_ended.clone();
      let on_ended = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
        is_playing.set(false);
        playback_progress.set(0.0);
        bar_heights.set(IDLE_HEIGHTS);
        // Clean up analysis
        if let Some(ref state) = *analysis_end_ref.borrow() {
          state.cleanup();
        }
        *analysis_end_ref.borrow_mut() = None;
        // Revoke the object URL to free memory
        let _ = web_sys::Url::revoke_object_url(&url_cleanup);
        audio_element.set(None);
      });
      audio.set_onended(Some(on_ended.as_ref().unchecked_ref()));
      on_ended.forget();

      let _ = audio.play().unwrap();
      audio_element.set(Some(audio));
      is_playing.set(true);
    }
  };

  // Keyboard handler for accessibility
  let handle_play_pause_clone = handle_play_pause.clone();
  let handle_keydown = move |ev: web_sys::KeyboardEvent| {
    if ev.key() == "Enter" || ev.key() == " " {
      ev.prevent_default();
      let mouse_ev = web_sys::MouseEvent::new("click").unwrap();
      handle_play_pause_clone(mouse_ev);
    }
  };

  view! {
    <div
      class="message-voice"
      tabindex=0
      role="button"
      aria-label=move || {
        if is_playing.get() {
          t_string!(i18n, chat_voice_pause)
        } else {
          t_string!(i18n, chat_voice_play)
        }
      }
      on:click=handle_play_pause
      on:keydown=handle_keydown
    >
      // Play/Pause button
      <button class=move || {
        if is_playing.get() { "voice-play-btn playing" } else { "voice-play-btn" }
      }>
        {move || if is_playing.get() { "⏸" } else { "▶" }}
      </button>

      // Waveform with progress overlay — bar heights driven by real-time audio analysis
      <div class="voice-waveform-container">
        <div
          class="voice-progress-fill"
          style=move || format!("width: {}%", playback_progress.get() * 100.0)
        ></div>
        <div class="voice-waveform">
          {move || {
            let heights = bar_heights.get();
            heights
              .iter()
              .map(|&h| {
                view! {
                  <div
                    class="voice-bar"
                    style=format!("height: {h:.1}px")
                  ></div>
                }
              })
              .collect::<Vec<_>>()
          }}
        </div>
      </div>

      // Duration display
      <span class="voice-duration">{format!("{duration_sec:.1}s")}</span>
    </div>
  }
}
