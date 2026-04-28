//! WASM-only capture pipeline for the voice recorder.
//!
//! This module is gated on `#[cfg(target_arch = "wasm32")]`. The
//! parent `voice_recorder::mod` owns the UI state and delegates
//! every browser-side side effect down here so the pure logic in
//! `waveform` stays native-testable.
//!
//! # Lifecycle
//!
//! * [`spawn_start_recording`] — async getUserMedia → MediaRecorder
//!   → AudioContext/AnalyserNode → rAF loop.
//! * [`spawn_stop_and_send`] — request `MediaRecorder::stop()`, wait
//!   for the final `dataavailable`, concatenate chunks, hand them to
//!   `ChatManager::send_voice`.
//! * [`cancel_recording`] — tear everything down without sending.

use super::waveform::{
  FINAL_SAMPLE_COUNT, MAX_DURATION_MS, RecordingState, WaveformAggregator, average_loudness,
};
use super::{CANVAS_HEIGHT, CANVAS_WIDTH, LIVE_BAR_WINDOW};
use crate::chat::ChatManager;
use crate::state::ConversationId;
use js_sys::{Array, ArrayBuffer, Reflect, Uint8Array};
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
  AudioContext, BlobEvent, BlobPropertyBag, HtmlCanvasElement, MediaRecorder, MediaRecorderOptions,
  MediaStream, MediaStreamAudioSourceNode, MediaStreamConstraints, Navigator, window,
};

/// Preferred MIME type for Opus-in-WebM capture.
///
/// All modern browsers accept this string; callers drop back to the
/// empty string if `MediaRecorder::isTypeSupported` reports `false`.
const MIME_TYPE: &str = "audio/webm;codecs=opus";

/// Time-slice (ms) passed to `MediaRecorder.start(slice)` so we get
/// periodic `dataavailable` events instead of waiting for the final
/// flush. 250 ms keeps memory bounded without fragmenting the clip.
const CHUNK_SLICE_MS: i32 = 250;

/// Per-peer resources allocated for the active recording.
pub(super) struct RecordingSession {
  pub(super) stream: MediaStream,
  pub(super) recorder: MediaRecorder,
  pub(super) audio_ctx: AudioContext,
  pub(super) _source: MediaStreamAudioSourceNode,
  pub(super) chunks: Rc<RefCell<Vec<Vec<u8>>>>,
  pub(super) aggregator: Rc<RefCell<WaveformAggregator>>,
  pub(super) start_ms: f64,
  pub(super) raf_handle: Rc<RefCell<Option<i32>>>,
  /// Kept so the closures are not GC'd until we call `close()`.
  pub(super) _on_data: Closure<dyn FnMut(BlobEvent)>,
  pub(super) _on_stop: Closure<dyn FnMut(web_sys::Event)>,
  pub(super) _raf_closure: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>>,
  /// Resolves when the final `dataavailable` event has fired after
  /// `MediaRecorder::stop()`. Used by `spawn_stop_and_send` to await
  /// the flush without polling.
  pub(super) on_stop_resolver: Rc<RefCell<Option<js_sys::Function>>>,
}

impl RecordingSession {
  /// Stop the live rAF loop, detach tracks, and close the audio
  /// context. Idempotent.
  pub(super) fn close(&self) {
    // Cancel RAF.
    if let Some(handle) = self.raf_handle.borrow_mut().take()
      && let Some(win) = window()
    {
      let _ = win.cancel_animation_frame(handle);
    }
    *self._raf_closure.borrow_mut() = None;

    // Stop every audio track so the browser red-dot goes away.
    let tracks = self.stream.get_audio_tracks();
    for i in 0..tracks.length() {
      if let Ok(track) = tracks.get(i).dyn_into::<web_sys::MediaStreamTrack>() {
        track.stop();
      }
    }

    // Close the audio context (fire-and-forget; the returned Promise
    // is ignored because the browser handles it asynchronously and
    // we do not care about the resolution).
    let _ = self.audio_ctx.close();
  }
}

/// Start a capture session.
///
/// Returns immediately; actual work runs on the microtask queue.
pub(super) fn spawn_start_recording(
  session_cell: Rc<RefCell<Option<RecordingSession>>>,
  state: RwSignal<RecordingState>,
  elapsed_ms: RwSignal<u32>,
  error_key: RwSignal<Option<String>>,
  canvas_ref: NodeRef<leptos::html::Canvas>,
) {
  wasm_bindgen_futures::spawn_local(async move {
    match start_recording_async(session_cell.clone(), canvas_ref, elapsed_ms).await {
      Ok(()) => {
        state.set(RecordingState::Recording);
      }
      Err(key) => {
        state.set(RecordingState::Idle);
        error_key.set(Some(key));
        // If partial resources were allocated, discard them.
        if let Some(s) = session_cell.borrow_mut().take() {
          s.close();
        }
      }
    }
  });
}

async fn start_recording_async(
  session_cell: Rc<RefCell<Option<RecordingSession>>>,
  canvas_ref: NodeRef<leptos::html::Canvas>,
  elapsed_ms: RwSignal<u32>,
) -> Result<(), String> {
  // 0. Feature detection -------------------------------------------------
  let win = window().ok_or_else(|| "unsupported".to_string())?;
  let navigator: Navigator = win.navigator();
  let media_devices = navigator
    .media_devices()
    .map_err(|_| "unsupported".to_string())?;
  if !is_type_supported(MIME_TYPE) {
    return Err("unsupported".to_string());
  }

  // 1. getUserMedia({audio:true}) --------------------------------------
  let constraints = MediaStreamConstraints::new();
  constraints.set_audio(&JsValue::TRUE);
  let promise = media_devices
    .get_user_media_with_constraints(&constraints)
    .map_err(|_| "capture_failed".to_string())?;
  let stream_js = JsFuture::from(promise)
    .await
    .map_err(|_| "permission_denied".to_string())?;
  let stream: MediaStream = stream_js
    .dyn_into()
    .map_err(|_| "capture_failed".to_string())?;

  // 2. Build MediaRecorder(opus) --------------------------------------
  let options = MediaRecorderOptions::new();
  options.set_mime_type(MIME_TYPE);
  let recorder = MediaRecorder::new_with_media_stream_and_media_recorder_options(&stream, &options)
    .map_err(|_| "capture_failed".to_string())?;

  // 3. Build AudioContext + AnalyserNode for live waveform -------------
  let audio_ctx = AudioContext::new().map_err(|_| "capture_failed".to_string())?;
  let analyser = audio_ctx
    .create_analyser()
    .map_err(|_| "capture_failed".to_string())?;
  analyser.set_fft_size(1024);
  let source = audio_ctx
    .create_media_stream_source(&stream)
    .map_err(|_| "capture_failed".to_string())?;
  let source_node: &web_sys::AudioNode = source.as_ref();
  let analyser_node: &web_sys::AudioNode = analyser.as_ref();
  source_node
    .connect_with_audio_node(analyser_node)
    .map_err(|_| "capture_failed".to_string())?;

  // 4. Wire up chunk aggregation ---------------------------------------
  let chunks: Rc<RefCell<Vec<Vec<u8>>>> = Rc::new(RefCell::new(Vec::new()));
  let aggregator = Rc::new(RefCell::new(WaveformAggregator::new()));
  let raf_handle: Rc<RefCell<Option<i32>>> = Rc::new(RefCell::new(None));
  let raf_closure_slot: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));

  let chunks_for_data = chunks.clone();
  let on_data = Closure::wrap(Box::new(move |ev: BlobEvent| {
    // `BlobEvent::data()` returns `Option<Blob>` (not a `Result`).
    // Some browsers fire the event with no payload on the final
    // flush — fall through silently in that case.
    let Some(data) = ev.data() else {
      return;
    };
    if data.size() <= 0.0 {
      return;
    }
    let bytes_fut = JsFuture::from(data.array_buffer());
    let chunks_clone = chunks_for_data.clone();
    wasm_bindgen_futures::spawn_local(async move {
      if let Ok(ab) = bytes_fut.await
        && let Ok(buffer) = ab.dyn_into::<ArrayBuffer>()
      {
        let u8arr = Uint8Array::new(&buffer);
        let mut bytes = vec![0u8; u8arr.length() as usize];
        u8arr.copy_to(&mut bytes);
        chunks_clone.borrow_mut().push(bytes);
      }
    });
  }) as Box<dyn FnMut(BlobEvent)>);
  recorder.set_ondataavailable(Some(on_data.as_ref().unchecked_ref()));

  // 5. onstop — resolve the stop-flush promise -------------------------
  let on_stop_resolver: Rc<RefCell<Option<js_sys::Function>>> = Rc::new(RefCell::new(None));
  let on_stop_resolver_for_cb = on_stop_resolver.clone();
  let on_stop = Closure::wrap(Box::new(move |_ev: web_sys::Event| {
    if let Some(resolver) = on_stop_resolver_for_cb.borrow_mut().take() {
      let _ = resolver.call0(&JsValue::NULL);
    }
  }) as Box<dyn FnMut(web_sys::Event)>);
  recorder.set_onstop(Some(on_stop.as_ref().unchecked_ref()));

  // 6. Start the recorder ---------------------------------------------
  recorder
    .start_with_time_slice(CHUNK_SLICE_MS)
    .map_err(|_| "capture_failed".to_string())?;

  // 7. Spin up the rAF loop for the live waveform ---------------------
  let start_ms = js_sys::Date::now();
  let aggregator_for_raf = aggregator.clone();
  let raf_handle_for_raf = raf_handle.clone();
  let raf_slot_for_raf = raf_closure_slot.clone();
  let session_for_raf = session_cell.clone();
  let canvas_for_raf = canvas_ref;
  let analyser_for_raf = analyser.clone();

  let raf_closure = Closure::wrap(Box::new(move |_ts: f64| {
    let now = js_sys::Date::now();
    let elapsed = ((now - start_ms).max(0.0)) as u32;

    // Always update the elapsed UI so the timer ticks even if the
    // aggregator throttles.
    elapsed_ms.set(elapsed);

    // Sample the spectrum into the aggregator (rate-limited by the
    // pure-logic `should_sample`).
    let bin_count = analyser_for_raf.frequency_bin_count();
    let mut freq_buf = vec![0u8; bin_count as usize];
    analyser_for_raf.get_byte_frequency_data(&mut freq_buf);
    let loudness = average_loudness(&freq_buf);
    {
      let mut agg = aggregator_for_raf.borrow_mut();
      if agg.should_sample(i64::from(elapsed)) {
        agg.push(loudness, i64::from(elapsed));
      }
    }

    // Paint the canvas.
    if let Some(canvas_el) = canvas_for_raf.get() {
      let html_canvas: &HtmlCanvasElement = canvas_el.as_ref();
      draw_waveform_bars(html_canvas, &aggregator_for_raf.borrow());
    }

    // Auto-stop at the 120 s ceiling.
    if elapsed >= MAX_DURATION_MS {
      // Request stop from outside the closure; the onstop handler
      // will resolve the await in `spawn_stop_and_send` if anyone
      // is waiting, otherwise the session will simply go Stopping
      // and the UI will flip through handle_stop_and_send.
      if let Some(session) = session_for_raf.borrow().as_ref()
        && session.recorder.state() == web_sys::RecordingState::Recording
      {
        let _ = session.recorder.stop();
      }
      return;
    }

    // Re-arm the RAF.
    if let Some(win) = window()
      && let Some(cb) = raf_slot_for_raf.borrow().as_ref()
    {
      if let Ok(handle) = win.request_animation_frame(cb.as_ref().unchecked_ref()) {
        *raf_handle_for_raf.borrow_mut() = Some(handle);
      }
    }
  }) as Box<dyn FnMut(f64)>);

  // Install the first RAF.
  if let Some(win) = window()
    && let Ok(handle) = win.request_animation_frame(raf_closure.as_ref().unchecked_ref())
  {
    *raf_handle.borrow_mut() = Some(handle);
  }
  *raf_closure_slot.borrow_mut() = Some(raf_closure);

  // 8. Commit the session ---------------------------------------------
  let session = RecordingSession {
    stream,
    recorder,
    audio_ctx,
    _source: source,
    chunks,
    aggregator,
    start_ms,
    raf_handle,
    _on_data: on_data,
    _on_stop: on_stop,
    _raf_closure: raf_closure_slot,
    on_stop_resolver,
  };
  *session_cell.borrow_mut() = Some(session);

  Ok(())
}

/// Check if a MIME type is supported by `MediaRecorder`.
fn is_type_supported(mime: &str) -> bool {
  let Some(win) = window() else {
    return false;
  };
  let Some(global) = win.dyn_ref::<web_sys::Window>() else {
    return false;
  };
  // Access the `MediaRecorder` constructor via Reflect so we can call
  // the static `isTypeSupported`.
  let Ok(ctor) = Reflect::get(global, &JsValue::from_str("MediaRecorder")) else {
    return false;
  };
  let Ok(fun) = Reflect::get(&ctor, &JsValue::from_str("isTypeSupported")) else {
    return false;
  };
  let Ok(func) = fun.dyn_into::<js_sys::Function>() else {
    return false;
  };
  func
    .call1(&ctor, &JsValue::from_str(mime))
    .ok()
    .and_then(|v| v.as_bool())
    .unwrap_or(false)
}

/// Draw the last [`LIVE_BAR_WINDOW`] samples as a bar chart.
fn draw_waveform_bars(canvas: &HtmlCanvasElement, aggregator: &WaveformAggregator) {
  // Fast path: aggregator has not captured anything yet.
  if aggregator.is_empty() {
    return;
  }
  let Ok(Some(ctx_js)) = canvas.get_context("2d") else {
    return;
  };
  let Ok(ctx) = ctx_js.dyn_into::<web_sys::CanvasRenderingContext2d>() else {
    return;
  };
  let w = f64::from(CANVAS_WIDTH);
  let h = f64::from(CANVAS_HEIGHT);
  ctx.clear_rect(0.0, 0.0, w, h);
  ctx.set_fill_style_str("#4f8eff");

  // Sanity check: the tail must never outrun the total sample count
  // the aggregator has collected. Use `samples()` + `len()` so any
  // future off-by-one in `tail()` is caught immediately in dev
  // builds.
  let tail = aggregator.tail(LIVE_BAR_WINDOW);
  debug_assert!(
    tail.len() <= aggregator.len() && aggregator.samples().len() == aggregator.len(),
    "aggregator tail window exceeded total sample count"
  );
  let bar_gap = 1.0_f64;
  let bar_width = (w / LIVE_BAR_WINDOW as f64 - bar_gap).max(1.0);
  let start_x = w - (tail.len() as f64 * (bar_width + bar_gap));
  for (i, &sample) in tail.iter().enumerate() {
    let bar_h = (f64::from(sample) / 255.0) * h;
    let x = start_x + i as f64 * (bar_width + bar_gap);
    let y = (h - bar_h) / 2.0;
    ctx.fill_rect(x, y, bar_width, bar_h.max(1.0));
  }
}

/// Stop the recorder and hand the clip to `ChatManager::send_voice`.
pub(super) fn spawn_stop_and_send(
  session_cell: Rc<RefCell<Option<RecordingSession>>>,
  state: RwSignal<RecordingState>,
  visible: RwSignal<bool>,
  error_key: RwSignal<Option<String>>,
  manager: ChatManager,
  conv: ConversationId,
) {
  wasm_bindgen_futures::spawn_local(async move {
    // Snapshot everything we need up-front so we do not hold the
    // RefCell across `.await`. Bind the Ref to a `let` so its
    // lifetime extends across the tuple construction below (the
    // previous `let Some(_) = cell.borrow().as_ref() else { ... };`
    // pattern dropped the temporary Ref before the tuple fields
    // finished cloning, triggering E0716).
    let snapshot = {
      let guard = session_cell.borrow();
      let Some(session) = guard.as_ref() else {
        state.set(RecordingState::Idle);
        visible.set(false);
        return;
      };
      (
        session.recorder.clone(),
        session.chunks.clone(),
        session.aggregator.clone(),
        session.start_ms,
        session.on_stop_resolver.clone(),
      )
    };
    let (recorder, chunks, aggregator, start_ms, on_stop_resolver) = snapshot;

    // Build a Promise that resolves once `onstop` fires, then request
    // the stop if the recorder is still running.
    let stop_promise = js_sys::Promise::new(&mut |resolve, _reject| {
      *on_stop_resolver.borrow_mut() = Some(resolve);
    });
    if recorder.state() == web_sys::RecordingState::Recording {
      let _ = recorder.stop();
    } else {
      // Already stopped (e.g. the 120 s auto-stop fired first). Resolve
      // immediately so we don't deadlock.
      if let Some(resolver) = on_stop_resolver.borrow_mut().take() {
        let _ = resolver.call0(&JsValue::NULL);
      }
    }
    let _ = JsFuture::from(stop_promise).await;

    // Concatenate chunks.
    let mut audio_data: Vec<u8> = Vec::new();
    for chunk in chunks.borrow_mut().drain(..) {
      audio_data.extend_from_slice(&chunk);
    }

    let duration_ms = {
      let now = js_sys::Date::now();
      ((now - start_ms).max(0.0) as u32).min(MAX_DURATION_MS)
    };
    let waveform_payload = aggregator.borrow().downsample_final();
    debug_assert_eq!(waveform_payload.len(), FINAL_SAMPLE_COUNT);

    // Build a blob URL so the sender-side bubble can play its own
    // recording immediately while the wire message is still in flight.
    let object_url = object_url_from_bytes(&audio_data).unwrap_or_default();

    // Hand off to ChatManager; drop the session regardless of result.
    if audio_data.is_empty() || duration_ms == 0 {
      error_key.set(Some("capture_failed".into()));
    } else {
      let _ = manager.send_voice(conv, audio_data, duration_ms, waveform_payload, object_url);
    }

    if let Some(session) = session_cell.borrow_mut().take() {
      session.close();
    }
    state.set(RecordingState::Idle);
    visible.set(false);
  });
}

/// Tear down an active session without sending anything.
pub(super) fn cancel_recording(session_cell: Rc<RefCell<Option<RecordingSession>>>) {
  let taken = session_cell.borrow_mut().take();
  if let Some(session) = taken {
    if session.recorder.state() == web_sys::RecordingState::Recording {
      let _ = session.recorder.stop();
    }
    session.close();
  }
}

/// Best-effort blob-URL construction. Returns an empty string if
/// `URL.createObjectURL` is unavailable.
fn object_url_from_bytes(bytes: &[u8]) -> Option<String> {
  let uint8 = Uint8Array::new_with_length(bytes.len() as u32);
  uint8.copy_from(bytes);
  let parts = Array::new();
  parts.push(&uint8.buffer());
  let bag = BlobPropertyBag::new();
  bag.set_type(MIME_TYPE);
  let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &bag).ok()?;
  web_sys::Url::create_object_url_with_blob(&blob).ok()
}
