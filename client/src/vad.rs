//! VAD (Voice Activity Detection) speaker detection
//!
//! Uses Web Audio API's `AnalyserNode` to analyze audio stream volume,
//! periodically detects speaking state and updates global `VadState`.

use std::collections::HashMap;

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

use crate::state::VadState;

#[allow(unused_imports)]
use wasm_bindgen::JsValue;

/// Volume threshold: values above this are considered speaking (RMS value in 0-255 range)
const SPEAKING_THRESHOLD: f64 = 25.0;

/// Detection interval (milliseconds)
const DETECTION_INTERVAL_MS: i32 = 100;

/// Silence frame threshold: number of consecutive frames below threshold before
/// determining speech has stopped (debounce)
const SILENCE_FRAMES_THRESHOLD: u32 = 5;

/// VAD analyser for a single audio stream
struct VadAnalyser {
  /// Web Audio context
  _context: web_sys::AudioContext,
  /// Analyser node
  analyser: web_sys::AnalyserNode,
  /// Time-domain data buffer (u8 slice, length = fftSize / 2)
  buffer: Vec<u8>,
  /// Consecutive silence frame count
  silence_frames: u32,
}

/// Global VAD manager
///
/// Manages VAD analysers for all audio streams, periodically polls to detect speaking state.
#[derive(Clone)]
pub struct VadManager {
  /// Analyser per user (user_id -> VadAnalyser)
  analysers: StoredValue<HashMap<String, VadAnalyser>>,
  /// Polling timer ID
  timer_id: StoredValue<Option<i32>>,
}

impl VadManager {
  /// Create and provide to context
  pub fn provide() {
    let manager = Self {
      analysers: StoredValue::new(HashMap::new()),
      timer_id: StoredValue::new(None),
    };
    provide_context(manager);
  }

  /// Get from context
  pub fn use_manager() -> Self {
    use_context::<Self>().expect("VadManager not provided")
  }

  /// Add VAD analysis for a user's audio stream
  ///
  /// Extracts audio tracks from `MediaStream` and creates an `AnalyserNode` for volume analysis.
  pub fn add_stream(&self, user_id: &str, stream: &web_sys::MediaStream) {
    // Check if there are audio tracks
    let audio_tracks = stream.get_audio_tracks();
    if audio_tracks.length() == 0 {
      web_sys::console::warn_1(
        &format!("[VAD] User {user_id}'s stream has no audio tracks, skipping").into(),
      );
      return;
    }

    // Create AudioContext
    let context = match web_sys::AudioContext::new() {
      Ok(ctx) => ctx,
      Err(e) => {
        web_sys::console::error_1(&format!("[VAD] Failed to create AudioContext: {e:?}").into());
        return;
      }
    };

    // Create AnalyserNode
    let analyser = match context.create_analyser() {
      Ok(a) => a,
      Err(e) => {
        web_sys::console::error_1(&format!("[VAD] Failed to create AnalyserNode: {e:?}").into());
        return;
      }
    };
    analyser.set_fft_size(256);
    analyser.set_smoothing_time_constant(0.5);

    // Create MediaStreamAudioSourceNode and connect to AnalyserNode
    let source = match context.create_media_stream_source(stream) {
      Ok(s) => s,
      Err(e) => {
        web_sys::console::error_1(
          &format!("[VAD] Failed to create MediaStreamSource: {e:?}").into(),
        );
        return;
      }
    };

    if let Err(e) = source.connect_with_audio_node(&analyser) {
      web_sys::console::error_1(&format!("[VAD] Failed to connect AnalyserNode: {e:?}").into());
      return;
    }

    // Note: do not connect to destination to avoid echo
    let buffer_length = analyser.frequency_bin_count() as usize;
    let buffer = vec![0u8; buffer_length];

    let vad_analyser = VadAnalyser {
      _context: context,
      analyser,
      buffer,
      silence_frames: 0,
    };

    let uid = user_id.to_string();
    self.analysers.update_value(|map| {
      map.insert(uid.clone(), vad_analyser);
    });

    web_sys::console::log_1(&format!("[VAD] Added audio analysis for user {uid}").into());

    // If this is the first analyser, start polling
    self.ensure_polling();
  }

  /// Remove VAD analysis for a user
  pub fn remove_stream(&self, user_id: &str) {
    self.analysers.update_value(|map| {
      map.remove(user_id);
    });

    // Remove from VadState
    if let Some(vad_state) = use_context::<RwSignal<VadState>>() {
      vad_state.update(|s| {
        s.speaking_users.remove(user_id);
        s.volume_levels.remove(user_id);
      });
    }

    // If no analysers remain, stop polling
    let is_empty = self.analysers.with_value(|map| map.is_empty());
    if is_empty {
      self.stop_polling();
    }

    web_sys::console::log_1(&format!("[VAD] Removed audio analysis for user {user_id}").into());
  }

  /// Remove all VAD analyses
  pub fn remove_all(&self) {
    self.analysers.update_value(|map| map.clear());
    self.stop_polling();

    if let Some(vad_state) = use_context::<RwSignal<VadState>>() {
      vad_state.update(|s| {
        s.speaking_users.clear();
        s.volume_levels.clear();
      });
    }
  }

  /// Ensure the polling timer is running
  fn ensure_polling(&self) {
    let has_timer = self.timer_id.with_value(|id| id.is_some());
    if has_timer {
      return;
    }

    let self_clone = self.clone();
    let cb = Closure::<dyn Fn()>::new(move || {
      self_clone.poll_all();
    });

    let id = web_sys::window().map_or(0, |w| {
      w.set_interval_with_callback_and_timeout_and_arguments_0(
        cb.as_ref().unchecked_ref(),
        DETECTION_INTERVAL_MS,
      )
      .unwrap_or(0)
    });
    cb.forget();

    self.timer_id.set_value(Some(id));
  }

  /// Stop the polling timer
  fn stop_polling(&self) {
    if let Some(id) = self.timer_id.get_value() {
      if let Some(window) = web_sys::window() {
        window.clear_interval_with_handle(id);
      }
      self.timer_id.set_value(None);
    }
  }

  /// Poll all analysers and update speaking state
  fn poll_all(&self) {
    let Some(vad_state) = use_context::<RwSignal<VadState>>() else {
      return;
    };

    // Collect volume data for all users
    let mut updates: Vec<(String, f64, bool)> = Vec::new();

    self.analysers.update_value(|map| {
      for (user_id, analyser) in map.iter_mut() {
        // Get time-domain data
        analyser
          .analyser
          .get_byte_time_domain_data(&mut analyser.buffer);

        // Calculate RMS volume
        let length = analyser.buffer.len();
        let mut sum = 0.0_f64;
        for &val_u8 in &analyser.buffer {
          let val = val_u8 as f64 - 128.0;
          sum += val * val;
        }
        let rms = (sum / length as f64).sqrt();

        // Normalize to 0-100 range
        let volume = (rms / 128.0 * 100.0).min(100.0);

        // Speaking state detection (with debounce)
        let is_speaking = if rms > SPEAKING_THRESHOLD {
          analyser.silence_frames = 0;
          true
        } else {
          analyser.silence_frames += 1;
          analyser.silence_frames < SILENCE_FRAMES_THRESHOLD
        };

        updates.push((user_id.clone(), volume, is_speaking));
      }
    });

    // Batch update VadState (trigger signal update only once)
    if !updates.is_empty() {
      vad_state.update(|s| {
        for (user_id, volume, is_speaking) in updates {
          s.volume_levels.insert(user_id.clone(), volume);
          if is_speaking {
            s.speaking_users.insert(user_id);
          } else {
            s.speaking_users.remove(&user_id);
          }
        }
      });
    }
  }
}
