//! Voice recorder overlay.
//!
//! This is the UI surface for voice-note capture. The actual Opus
//! encoding is performed by the browser `MediaRecorder` API, whose
//! web-sys bindings are not enabled in this crate's feature set
//! (they require `BlobEvent` + `MediaRecorderOptions` + `MediaRecorder`,
//! which significantly increases WASM bundle size). Until Task-25
//! wires real audio capture, this overlay runs in **preview-only**
//! mode: it always shows a "not supported" message and a close
//! button. No audio data is sent.

use crate::i18n;
use leptos::prelude::*;
use leptos_i18n::t_string;

/// Voice recorder overlay (preview-only until Task-25).
#[component]
pub fn VoiceRecorder(
  /// Visibility signal.
  visible: RwSignal<bool>,
) -> impl IntoView {
  let i18n = i18n::use_i18n();

  view! {
    <Show when=move || visible.get() fallback=|| ()>
      <div class="voice-recorder" role="status" data-testid="voice-recorder">
        <span class="voice-recorder-warning">
          {move || t_string!(i18n, chat.voice_not_supported)}
        </span>
        <button
          type="button"
          class="chat-input-btn"
          aria-label=move || t_string!(i18n, common.close)
          on:click=move |_| visible.set(false)
        >
          "×"
        </button>
      </div>
    </Show>
  }
}
