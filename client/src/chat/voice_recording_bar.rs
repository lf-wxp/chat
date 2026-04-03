//! Voice recording status bar component
//!
//! Displays recording state with real-time waveform driven by audio levels.

use leptos::prelude::*;
use leptos_i18n::t;

use crate::i18n::*;

use super::voice_recorder::WAVE_BAR_COUNT;

/// Maximum bar height in pixels
const BAR_MAX_HEIGHT: f32 = 28.0;
/// Minimum bar height in pixels
const BAR_MIN_HEIGHT: f32 = 4.0;

/// Voice recording status bar with real-time waveform and cancel hint
#[component]
pub fn VoiceRecordingBar(
  /// Whether currently recording
  is_recording: RwSignal<bool>,
  /// Recording duration in milliseconds
  recording_duration: RwSignal<u32>,
  /// Whether the user is in the cancel zone (sliding up)
  voice_cancel_hint: RwSignal<bool>,
  /// Real-time audio levels [0.0..1.0] for each waveform bar
  voice_levels: RwSignal<[f32; WAVE_BAR_COUNT]>,
) -> impl IntoView {
  let i18n = use_i18n();

  move || {
    if !is_recording.get() {
      return view! { <div class="voice-rec-hidden"></div> }.into_any();
    }

    let in_cancel_zone = voice_cancel_hint.get();
    let dur = recording_duration.get();
    let secs = dur / 1000;
    let ms = (dur % 1000) / 100;

    if in_cancel_zone {
      // Show cancel hint when user slides up
      view! {
        <div class="voice-recording-bar voice-cancel-active">
          <span class="voice-cancel-icon">"↑"</span>
          <span class="voice-cancel-label">{t!(i18n, chat_voice_release_cancel)}</span>
        </div>
      }.into_any()
    } else {
      // Normal recording state with real-time waveform
      let levels = voice_levels.get();

      view! {
        <div class="voice-recording-bar">
          <span class="voice-rec-dot"></span>
          <span class="voice-rec-label">{t!(i18n, chat_recording)}</span>
          <span class="voice-rec-time">{format!("{}:{:02}.{}", secs / 60, secs % 60, ms)}</span>
          <div class="voice-rec-wave">
            {levels
              .iter()
              .map(|&level| {
                let height = BAR_MIN_HEIGHT + level * (BAR_MAX_HEIGHT - BAR_MIN_HEIGHT);
                view! {
                  <div
                    class="voice-rec-bar"
                    style=format!("height: {height:.1}px")
                  ></div>
                }
              })
              .collect::<Vec<_>>()
            }
          </div>
          <span class="voice-rec-hint">{t!(i18n, chat_voice_slide_cancel)}</span>
        </div>
      }.into_any()
    }
  }
}
