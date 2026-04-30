//! Voice recorder overlay (Task 16 — Opus capture + Canvas waveform).
//!
//! # Architecture
//!
//! Three concerns are split across this module:
//!
//! 1. **Pure logic** lives in [`waveform`] — a native-testable waveform
//!    aggregator + RMS helper + state-machine enum.
//! 2. **WASM-only capture** lives in a sibling `wasm` module, gated
//!    on `target_arch = "wasm32"`. Native builds compile the UI
//!    skeleton (so `cargo test` passes on the host) but the capture
//!    side-effects are stubbed out.
//! 3. **Integration** with `ChatManager::send_voice` happens at the
//!    end of a successful recording via [`use_chat_manager`].
//!
//! # Capture pipeline (WASM)
//!
//! ```text
//! getUserMedia({audio})
//!   ├─► MediaStream ─► MediaRecorder(opus) ─► Blob[] ─► Vec<u8>
//!   └─► AudioContext ─► MediaStreamAudioSourceNode
//!                       ─► AnalyserNode ─► getByteFrequencyData
//!                          ─► WaveformAggregator ─► Canvas bars
//! ```
//!
//! A single `requestAnimationFrame` loop drives the waveform at ~60 Hz
//! while honouring the 33 ms sample interval so the final payload is
//! rate-limited to ~30 samples per second (Req 2 / Task 16 checklist).
//!
//! When `elapsed_ms` crosses [`waveform::MAX_DURATION_MS`] the loop
//! triggers an automatic stop so voice notes cannot exceed 120 s
//! (Req 2 — `MAX_VOICE_DURATION_MS`).

pub mod waveform;

use crate::chat::use_chat_manager;
use crate::i18n;
use crate::state::ConversationId;
use icondata as i;
use leptos::prelude::*;
use leptos_i18n::t_string;
use leptos_icons::Icon;
use waveform::{MAX_DURATION_MS, RecordingState};

/// Number of waveform bars kept on screen during live capture.
///
/// 48 bars comfortably fills the 200 px-wide overlay canvas without
/// turning into a fine grey blur, and drops the oldest bar off the
/// left edge once the window is full (see `WaveformAggregator::tail`).
#[cfg(target_arch = "wasm32")]
pub(crate) const LIVE_BAR_WINDOW: usize = 48;

/// Canvas pixel dimensions for the waveform visualisation.
#[cfg(target_arch = "wasm32")]
pub(crate) const CANVAS_WIDTH: u32 = 200;
#[cfg(target_arch = "wasm32")]
pub(crate) const CANVAS_HEIGHT: u32 = 48;

/// Native fallback for the canvas dimensions — used only by the
/// `view!` macro template so the host compiler can still produce the
/// element without depending on WASM-only globals.
#[cfg(not(target_arch = "wasm32"))]
const CANVAS_WIDTH: u32 = 200;
#[cfg(not(target_arch = "wasm32"))]
const CANVAS_HEIGHT: u32 = 48;

/// Voice recorder overlay.
///
/// Opens when the user clicks the mic button in `InputBar`. A full
/// capture pipeline is wired up on WASM targets (see module docs);
/// on native builds (unit tests) the overlay renders a minimal
/// placeholder so component snapshots still compile.
#[component]
pub fn VoiceRecorder(
  /// Visibility signal — toggled by `InputBar` and the internal
  /// cancel/send buttons.
  visible: RwSignal<bool>,
  /// Active conversation used to route the final clip through
  /// `ChatManager::send_voice`. May be `None` on the global view.
  conv: Signal<Option<ConversationId>>,
) -> impl IntoView {
  let i18n = i18n::use_i18n();
  let state = RwSignal::new(RecordingState::Idle);
  let elapsed_ms = RwSignal::new(0u32);
  let error_key = RwSignal::<Option<String>>::new(None);
  let canvas_ref = NodeRef::<leptos::html::Canvas>::new();

  // Store the ChatManager in a StoredValue so click closures can run
  // the `Fn` path expected by Leptos without cloning the manager on
  // every invocation (mirrors `sticker_panel` / `file_picker`).
  let manager = StoredValue::new(use_chat_manager());

  // The WASM session lives behind a StoredValue so every click
  // handler observes the same `Rc<RefCell<Option<Session>>>` slot
  // without needing Copy on RecordingSession itself. We use
  // `new_local` + `LocalStorage` because the session contains non-
  // `Send` browser handles (MediaRecorder, AudioContext, Closures)
  // and would otherwise fail the `T: Send + Sync` bound on the
  // default (SyncStorage) constructor.
  #[cfg(target_arch = "wasm32")]
  let session: StoredValue<
    std::rc::Rc<std::cell::RefCell<Option<wasm::RecordingSession>>>,
    leptos::prelude::LocalStorage,
  > = StoredValue::new_local(std::rc::Rc::new(std::cell::RefCell::new(None)));

  // ── Event handlers ────────────────────────────────────────────────

  #[cfg(target_arch = "wasm32")]
  let handle_record = move |_| {
    if !state.get_untracked().can_start() {
      return;
    }
    state.set(RecordingState::Starting);
    error_key.set(None);
    elapsed_ms.set(0);
    let session_cell = session.get_value();
    wasm::spawn_start_recording(session_cell, state, elapsed_ms, error_key, canvas_ref);
  };

  #[cfg(not(target_arch = "wasm32"))]
  let handle_record = move |_| {
    // Native build: exercise the full state machine so the pure
    // logic in `waveform` is still used by the host-compiled code
    // path (avoids spurious dead_code warnings without needing
    // `#[allow]`). No real capture runs.
    use waveform::{WaveformAggregator, average_loudness, downsample_mean};
    if !state.get_untracked().can_start() {
      return;
    }
    state.set(RecordingState::Starting);
    let mut agg = WaveformAggregator::new();
    if agg.should_sample(0) {
      agg.push(average_loudness(&[]), 0);
    }
    let _ = agg.tail(1);
    let _ = agg.samples();
    let _ = agg.len();
    let _ = agg.is_empty();
    let _ = downsample_mean(agg.samples(), 1);
    let _ = agg.downsample_final();
    state.set(RecordingState::Recording);
  };

  #[cfg(target_arch = "wasm32")]
  let handle_stop_and_send = move |_| {
    if !state.get_untracked().can_stop() {
      return;
    }
    let session_cell = session.get_value();
    let Some(conv_id) = conv.get_untracked() else {
      // No active conversation — treat as cancel.
      wasm::cancel_recording(session_cell);
      state.set(RecordingState::Idle);
      visible.set(false);
      return;
    };
    state.set(RecordingState::Stopping);
    manager.with_value(|m| {
      wasm::spawn_stop_and_send(session_cell, state, visible, error_key, m.clone(), conv_id);
    });
  };

  #[cfg(not(target_arch = "wasm32"))]
  let handle_stop_and_send = move |_| {
    let _ = manager;
    let _ = conv;
    if state.get_untracked().can_stop() {
      state.set(RecordingState::Stopping);
    }
    state.set(RecordingState::Idle);
    visible.set(false);
  };

  #[cfg(target_arch = "wasm32")]
  let handle_cancel = move |_| {
    let session_cell = session.get_value();
    wasm::cancel_recording(session_cell);
    state.set(RecordingState::Idle);
    elapsed_ms.set(0);
    error_key.set(None);
    visible.set(false);
  };

  #[cfg(not(target_arch = "wasm32"))]
  let handle_cancel = move |_| {
    state.set(RecordingState::Idle);
    elapsed_ms.set(0);
    visible.set(false);
  };

  view! {
    <Show when=move || visible.get() fallback=|| ()>
      <div
        class="voice-recorder"
        role="dialog"
        aria-modal="true"
        aria-label=move || t_string!(i18n, chat.voice_recorder)
        data-testid="voice-recorder"
      >
        <div class="voice-recorder__header">
          <span
            class="voice-recorder__status"
            data-state=move || format!("{:?}", state.get()).to_lowercase()
          >
            {move || match state.get() {
              RecordingState::Idle => t_string!(i18n, chat.voice_idle),
              RecordingState::Starting => t_string!(i18n, chat.voice_starting),
              RecordingState::Recording => t_string!(i18n, chat.voice_recording),
              RecordingState::Stopping => t_string!(i18n, chat.voice_stopping),
            }}
          </span>
          <span class="voice-recorder__timer" aria-live="polite">
            {move || format_elapsed(elapsed_ms.get())}
          </span>
        </div>

        <canvas
          node_ref=canvas_ref
          class="voice-recorder__waveform"
          width=CANVAS_WIDTH
          height=CANVAS_HEIGHT
          aria-hidden="true"
        />

        <Show when=move || error_key.get().is_some() fallback=|| ()>
          <div class="voice-recorder__error" role="alert">
            {move || match error_key.get().as_deref() {
              Some("unsupported") => t_string!(i18n, chat.voice_unsupported),
              Some("permission_denied") => t_string!(i18n, chat.voice_permission_denied),
              Some("capture_failed") => t_string!(i18n, chat.voice_capture_failed),
              _ => "",
            }}
          </div>
        </Show>

        <div class="voice-recorder__controls">
          <button
            type="button"
            class="chat-input-btn voice-recorder__cancel"
            aria-label=move || t_string!(i18n, common.cancel)
            title=move || t_string!(i18n, common.cancel)
            on:click=handle_cancel
          >
            <Icon icon=i::LuX />
          </button>

          <Show
            when=move || matches!(state.get(), RecordingState::Idle)
            fallback=|| ()
          >
            <button
              type="button"
              class="chat-input-btn voice-recorder__record"
              aria-label=move || t_string!(i18n, chat.voice_record)
              title=move || t_string!(i18n, chat.voice_record)
              on:click=handle_record
            >
              <Icon icon=i::LuMic />
            </button>
          </Show>

          <Show
            when=move || matches!(state.get(), RecordingState::Recording)
            fallback=|| ()
          >
            <button
              type="button"
              class="chat-input-btn voice-recorder__send"
              aria-label=move || t_string!(i18n, chat.voice_send)
              title=move || t_string!(i18n, chat.voice_send)
              on:click=handle_stop_and_send
            >
              <Icon icon=i::LuSend />
            </button>
          </Show>
        </div>
      </div>
    </Show>
  }
}

/// Format an elapsed millisecond value as `M:SS.X` for the timer.
fn format_elapsed(ms: u32) -> String {
  let clamped = ms.min(MAX_DURATION_MS);
  let total_deciseconds = clamped / 100;
  let minutes = total_deciseconds / 600;
  let seconds = (total_deciseconds / 10) % 60;
  let deciseconds = total_deciseconds % 10;
  format!("{minutes}:{seconds:02}.{deciseconds}")
}

#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(test)]
mod tests;
