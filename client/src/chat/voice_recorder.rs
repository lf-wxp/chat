//! Voice recording functionality for chat
//!
//! Manages MediaRecorder API for recording voice messages,
//! with real-time audio level analysis via Web Audio API AnalyserNode.

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::{GetUntracked, Set, Update};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

/// Number of waveform bars displayed during recording
pub const WAVE_BAR_COUNT: usize = 5;

/// Global voice recorder state:
/// (MediaRecorder, timer_id, AudioContext, AnalyserNode, rAF_id)
type RecorderState = (
  web_sys::MediaRecorder,
  i32,
  web_sys::AudioContext,
  web_sys::AnalyserNode,
  Rc<RefCell<i32>>, // requestAnimationFrame ID (mutable for cancel)
);

thread_local! {
  /// Global voice recorder state
  pub static VOICE_RECORDER: RefCell<Option<RecorderState>> = const { RefCell::new(None) };
}

/// Start the requestAnimationFrame loop to sample audio levels from the AnalyserNode.
///
/// Writes `WAVE_BAR_COUNT` normalized [0.0, 1.0] amplitude values into `voice_levels`.
fn start_level_monitor(
  analyser: &web_sys::AnalyserNode,
  voice_levels: leptos::prelude::RwSignal<[f32; WAVE_BAR_COUNT]>,
  raf_id: &Rc<RefCell<i32>>,
) {
  let analyser = analyser.clone();
  let raf_id = raf_id.clone();

  // fftSize = 256 → frequencyBinCount = 128
  analyser.set_fft_size(256);
  analyser.set_smoothing_time_constant(0.6);

  let bin_count = analyser.frequency_bin_count() as usize; // 128

  // Shared buffer for getByteFrequencyData
  let buf = Rc::new(RefCell::new(vec![0u8; bin_count]));

  // Recursive rAF closure via Rc<RefCell<Option<Closure>>>
  let closure: Rc<RefCell<Option<wasm_bindgen::closure::Closure<dyn Fn()>>>> =
    Rc::new(RefCell::new(None));
  let closure_clone = closure.clone();

  // Clone raf_id for the closure; the original stays available after
  let raf_id_inner = raf_id.clone();
  let cb = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
    let mut data = buf.borrow_mut();
    analyser.get_byte_frequency_data(&mut data);

    // Divide frequency bins into WAVE_BAR_COUNT bands and take average of each
    let band_size = data.len() / WAVE_BAR_COUNT;
    let mut levels = [0.0f32; WAVE_BAR_COUNT];
    for (i, level) in levels.iter_mut().enumerate() {
      let start = i * band_size;
      let end = (start + band_size).min(data.len());
      let sum: u32 = data[start..end].iter().map(|&v| v as u32).sum();
      let avg = sum as f32 / (end - start) as f32;
      // Normalize 0..255 → 0.0..1.0, with a minimum floor so bars are always visible
      *level = (avg / 255.0).max(0.08);
    }
    voice_levels.set(levels);

    // Schedule next frame
    if let Some(ref c) = *closure_clone.borrow() {
      let id = web_sys::window()
        .unwrap()
        .request_animation_frame(c.as_ref().unchecked_ref())
        .unwrap_or(0);
      *raf_id_inner.borrow_mut() = id;
    }
  });

  // Kick off the first frame
  let id = web_sys::window()
    .unwrap()
    .request_animation_frame(cb.as_ref().unchecked_ref())
    .unwrap_or(0);
  *raf_id.borrow_mut() = id;

  // Store the closure so it isn't dropped
  *closure.borrow_mut() = Some(cb);
  // Intentionally leak the Rc to keep the closure alive; it will be cleaned up
  // when stop/cancel drops the AnalyserNode and cancels rAF.
  std::mem::forget(closure);
}

/// Clean up audio analysis resources: cancel rAF and close AudioContext.
fn cleanup_analyser(audio_ctx: &web_sys::AudioContext, raf_id: &Rc<RefCell<i32>>) {
  // Cancel the animation frame loop
  let id = *raf_id.borrow();
  if id != 0 {
    web_sys::window()
      .unwrap()
      .cancel_animation_frame(id)
      .ok();
    *raf_id.borrow_mut() = 0;
  }
  // Close the AudioContext to release resources
  let _ = audio_ctx.close();
}

/// Start voice recording
///
/// Requests microphone permission, creates MediaRecorder + AudioContext/AnalyserNode,
/// and handles recording lifecycle. Recording automatically stops after 60 seconds.
pub fn start_voice_recording(
  peer_id: String,
  is_recording: leptos::prelude::RwSignal<bool>,
  recording_duration: leptos::prelude::RwSignal<u32>,
  voice_levels: leptos::prelude::RwSignal<[f32; WAVE_BAR_COUNT]>,
  on_complete: impl Fn(String, Vec<u8>, u32) + 'static,
) {
  if is_recording.get_untracked() {
    return;
  }
  is_recording.set(true);
  recording_duration.set(0);
  voice_levels.set([0.08; WAVE_BAR_COUNT]);

  spawn_local(async move {
    let on_complete: Rc<RefCell<dyn Fn(String, Vec<u8>, u32)>> = Rc::new(RefCell::new(on_complete));

    // Get microphone permission
    let navigator = web_sys::window().unwrap().navigator();
    let media_devices = navigator.media_devices().unwrap();
    let constraints = web_sys::MediaStreamConstraints::new();
    constraints.set_audio(&true.into());
    constraints.set_video(&false.into());

    let stream_promise = media_devices
      .get_user_media_with_constraints(&constraints)
      .unwrap();
    let stream_js = wasm_bindgen_futures::JsFuture::from(stream_promise).await;
    let Ok(stream_val) = stream_js else {
      web_sys::console::error_1(&"Failed to get microphone".into());
      is_recording.set(false);
      return;
    };
    let stream: web_sys::MediaStream = stream_val.unchecked_into();

    // ── Web Audio API: AudioContext → AnalyserNode ──
    let audio_ctx = web_sys::AudioContext::new().unwrap();
    let analyser = audio_ctx.create_analyser().unwrap();
    let source = audio_ctx
      .create_media_stream_source(&stream)
      .unwrap();
    // Connect source → analyser (no need to connect to destination; we only read data)
    let _ = source.connect_with_audio_node(&analyser);

    // Start the rAF level monitor
    let raf_id = Rc::new(RefCell::new(0i32));
    start_level_monitor(&analyser, voice_levels, &raf_id);

    // ── MediaRecorder ──
    let options = web_sys::MediaRecorderOptions::new();
    if web_sys::MediaRecorder::is_type_supported("audio/webm;codecs=opus") {
      options.set_mime_type("audio/webm;codecs=opus");
    }
    let recorder =
      web_sys::MediaRecorder::new_with_media_stream_and_media_recorder_options(&stream, &options)
        .unwrap();

    // Collect audio data chunks
    let chunks: Rc<RefCell<Vec<web_sys::Blob>>> = Rc::new(RefCell::new(Vec::new()));
    let chunks_data = chunks.clone();
    let on_data = wasm_bindgen::closure::Closure::<dyn Fn(web_sys::BlobEvent)>::new(
      move |ev: web_sys::BlobEvent| {
        if let Some(blob) = ev.data() {
          chunks_data.borrow_mut().push(blob);
        }
      },
    );
    recorder.set_ondataavailable(Some(on_data.as_ref().unchecked_ref()));
    on_data.forget();

    // Handle recording stop: merge data and send voice message
    let chunks_stop = chunks.clone();
    let stream_stop = stream.clone();
    let on_complete_stop = on_complete.clone();
    let on_stop =
      wasm_bindgen::closure::Closure::<dyn Fn(web_sys::Event)>::new(move |_: web_sys::Event| {
        let blobs = chunks_stop.borrow();
        if blobs.is_empty() {
          is_recording.set(false);
          return;
        }

        let parts = js_sys::Array::new();
        for b in blobs.iter() {
          parts.push(b);
        }
        let combined = web_sys::Blob::new_with_blob_sequence(&parts).unwrap();

        // Stop all tracks
        for track in stream_stop.get_tracks().iter() {
          if let Some(t) = track.dyn_ref::<web_sys::MediaStreamTrack>() {
            t.stop();
          }
        }

        let duration_ms = recording_duration.get_untracked();
        let peer_id_send = peer_id.clone();
        let on_complete_load = on_complete_stop.clone();

        let reader = web_sys::FileReader::new().unwrap();
        let reader_clone = reader.clone();
        let onload = wasm_bindgen::closure::Closure::<dyn Fn(web_sys::Event)>::new(
          move |_: web_sys::Event| {
            if let Ok(result) = reader_clone.result()
              && let Some(buf) = result.dyn_ref::<js_sys::ArrayBuffer>()
            {
              let array = js_sys::Uint8Array::new(buf);
              let data = array.to_vec();
              (*on_complete_load.borrow())(peer_id_send.clone(), data, duration_ms.max(500));
            }
            is_recording.set(false);
          },
        );
        reader.set_onload(Some(onload.as_ref().unchecked_ref()));
        onload.forget();
        let _ = reader.read_as_array_buffer(&combined);
      });
    recorder.set_onstop(Some(on_stop.as_ref().unchecked_ref()));
    on_stop.forget();

    // Start recording
    let _ = recorder.start_with_time_slice(250);

    // Recording timer
    let recorder_timer = recorder.clone();
    let timer_cb = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
      recording_duration.update(|d| *d += 100);
      if recording_duration.get_untracked() >= 60_000 {
        let _ = recorder_timer.stop();
      }
    });
    let timer_id = web_sys::window()
      .unwrap()
      .set_interval_with_callback_and_timeout_and_arguments_0(
        timer_cb.as_ref().unchecked_ref(),
        100,
      )
      .unwrap();
    timer_cb.forget();

    // Store all state globally for stop/cancel
    VOICE_RECORDER.with(|cell| {
      *cell.borrow_mut() = Some((recorder, timer_id, audio_ctx, analyser, raf_id));
    });
  });
}

/// Stop voice recording (triggers onstop → sends the message)
///
/// Stops the MediaRecorder, clears the timer, and cleans up audio analysis.
pub fn stop_voice_recording() {
  VOICE_RECORDER.with(|cell| {
    if let Some((recorder, timer_id, audio_ctx, _analyser, raf_id)) = cell.borrow_mut().take() {
      web_sys::window()
        .unwrap()
        .clear_interval_with_handle(timer_id);
      cleanup_analyser(&audio_ctx, &raf_id);
      if recorder.state() == web_sys::RecordingState::Recording {
        let _ = recorder.stop();
      }
    }
  });
}

/// Cancel voice recording (discards data, does NOT send)
///
/// Stops the MediaRecorder, clears the timer, cleans up audio analysis,
/// and stops all media tracks without triggering the onstop handler.
pub fn cancel_voice_recording(is_recording: leptos::prelude::RwSignal<bool>) {
  VOICE_RECORDER.with(|cell| {
    if let Some((recorder, timer_id, audio_ctx, _analyser, raf_id)) = cell.borrow_mut().take() {
      web_sys::window()
        .unwrap()
        .clear_interval_with_handle(timer_id);
      cleanup_analyser(&audio_ctx, &raf_id);

      // Remove the onstop handler so data is NOT sent
      recorder.set_onstop(None);

      // Stop all media tracks to release the microphone
      let stream = recorder.stream();
      for track in stream.get_tracks().iter() {
        if let Some(t) = track.dyn_ref::<web_sys::MediaStreamTrack>() {
          t.stop();
        }
      }

      if recorder.state() == web_sys::RecordingState::Recording {
        let _ = recorder.stop();
      }

      is_recording.set(false);
      web_sys::console::log_1(&"Voice recording cancelled".into());
    }
  });
}
