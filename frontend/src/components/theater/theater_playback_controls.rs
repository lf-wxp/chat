//! Theater playback controls (Req 12.4).
//!
//! A compact toolbar that sits beneath the `<video>` surface. The UI
//! differs by role:
//!
//! * **Owner** — play/pause button, seek bar, volume slider,
//!   fullscreen toggle. All mutations operate on the live `<video>`
//!   element; the element's own `play` / `pause` / `timeupdate`
//!   events in [`TheaterVideoPlayer`](super::TheaterVideoPlayer)
//!   broadcast the change to viewers through `PlaybackProgress`.
//! * **Viewer** — read-only progress bar, volume slider, fullscreen
//!   toggle. Owner-only controls are hidden so the viewer cannot
//!   trigger mutations.
//!
//! The toolbar deliberately queries the `<video>` element through a
//! `NodeRef` passed in from the parent — this keeps the playback
//! state of truth inside the browser (which owns buffering / seeking
//! quirks) while the reactive `TheaterState` is only a projection.

use icondata as i;
use leptos::html;
use leptos::prelude::*;
use leptos_i18n::t_string;
use leptos_icons::Icon;
use web_sys::HtmlVideoElement;

use crate::i18n;
use crate::theater::{TheaterRole, format_timestamp, use_theater_state};

/// Playback controls toolbar.
#[component]
pub fn TheaterPlaybackControls(
  /// Reference to the `<video>` element owned by
  /// [`TheaterVideoPlayer`](super::TheaterVideoPlayer). The toolbar
  /// dispatches `play()` / `pause()` / `currentTime = …` calls
  /// against it.
  video_ref: NodeRef<html::Video>,
  /// Reference to the container that should enter fullscreen (the
  /// theater page root). The toolbar calls
  /// `requestFullscreen()` / `exitFullscreen()` on it.
  #[prop(optional)]
  fullscreen_target: Option<NodeRef<html::Section>>,
) -> impl IntoView {
  let state = use_theater_state();
  let i18n = i18n::use_i18n();

  let is_owner = move || state.my_role.get() == TheaterRole::Owner;
  let playback = state.playback;

  let current_label = move || format_timestamp(playback.get().current_time_ms);
  let duration_label = move || format_timestamp(playback.get().duration_ms);

  let progress_percent = move || {
    let snap = playback.get();
    if snap.duration_ms == 0 {
      0.0
    } else {
      (snap.current_time_ms as f64 / snap.duration_ms as f64) * 100.0
    }
  };

  // --- Play / pause (owner only) ------------------------------------------
  let handle_play_pause = move |_| {
    if !is_owner() {
      return;
    }
    let Some(el) = video_ref.get() else { return };
    let video: &HtmlVideoElement = el.as_ref();
    if video.paused() {
      // `play()` returns a Promise that may reject when autoplay is
      // blocked — we forward the failure silently; the `<video>`
      // element itself will dispatch the corresponding event.
      let _ = video.play();
    } else {
      let _ = video.pause();
    }
  };

  // --- Seek (owner only, via range input) ---------------------------------
  let handle_seek_input = move |ev: leptos::ev::Event| {
    if !is_owner() {
      return;
    }
    let Some(el) = video_ref.get() else { return };
    let raw = event_target_value(&ev);
    let Ok(percent) = raw.parse::<f64>() else {
      return;
    };
    let snap = playback.get_untracked();
    if snap.duration_ms == 0 {
      return;
    }
    let target_ms = (snap.duration_ms as f64 * (percent / 100.0)) as u64;
    el.set_current_time((target_ms as f64) / 1_000.0);
  };

  // --- Volume (always available) ------------------------------------------
  let volume_value = RwSignal::new(100_u32);
  let muted = RwSignal::new(false);

  let handle_volume_input = move |ev: leptos::ev::Event| {
    let raw = event_target_value(&ev);
    let Ok(v) = raw.parse::<u32>() else { return };
    let clamped = v.clamp(0, 100);
    volume_value.set(clamped);
    if let Some(el) = video_ref.get() {
      el.set_volume(f64::from(clamped) / 100.0);
      el.set_muted(clamped == 0);
      muted.set(clamped == 0);
    }
  };

  let handle_mute_toggle = move |_| {
    let Some(el) = video_ref.get() else { return };
    let next_muted = !muted.get();
    el.set_muted(next_muted);
    muted.set(next_muted);
  };

  // --- Fullscreen toggle --------------------------------------------------
  let handle_fullscreen = move |_| {
    let Some(target) = fullscreen_target else {
      return;
    };
    let Some(el) = target.get() else { return };
    let doc = web_sys::window().and_then(|w| w.document());
    let in_fullscreen = doc.as_ref().and_then(|d| d.fullscreen_element()).is_some();
    if in_fullscreen {
      if let Some(doc) = doc {
        doc.exit_fullscreen();
      }
      state.is_fullscreen.set(false);
    } else {
      // request_fullscreen returns a Promise which may reject when
      // the gesture is rejected; we ignore the failure — the browser
      // surfaces its own UI.
      let _ = el.request_fullscreen();
      state.is_fullscreen.set(true);
    }
  };

  let play_label = move || {
    if state.playback.get().is_paused {
      t_string!(i18n, theater.play).to_string()
    } else {
      t_string!(i18n, theater.pause).to_string()
    }
  };

  view! {
    <div
      class="theater-playback-controls"
      role="group"
      aria-label=move || t_string!(i18n, theater.seek_bar)
      data-testid="theater-playback-controls"
    >
      <Show when=is_owner>
        <button
          type="button"
          class="btn btn--icon theater-playback-controls__play"
          on:click=handle_play_pause
          aria-label=play_label
          data-testid="theater-play-pause"
        >
          <Show
            when=move || state.playback.get().is_paused
            fallback=|| view! { <Icon icon=i::LuPause /> }
          >
            <Icon icon=i::LuPlay />
          </Show>
        </button>
      </Show>

      <span class="theater-playback-controls__time" aria-hidden="true">
        {current_label}
      </span>

      <input
        class="theater-playback-controls__seek"
        type="range"
        min="0"
        max="100"
        step="0.1"
        prop:value=progress_percent
        on:input=handle_seek_input
        aria-label=move || t_string!(i18n, theater.seek_bar)
        disabled=move || !is_owner()
        data-testid="theater-seek-bar"
      />

      <span class="theater-playback-controls__time" aria-hidden="true">
        {duration_label}
      </span>

      <button
        type="button"
        class="btn btn--icon"
        on:click=handle_mute_toggle
        aria-label=move || {
          if muted.get() {
            t_string!(i18n, theater.unmute).to_string()
          } else {
            t_string!(i18n, theater.mute).to_string()
          }
        }
        data-testid="theater-mute-toggle"
      >
        <Show when=move || muted.get() fallback=|| view! { <Icon icon=i::LuVolume2 /> }>
          <Icon icon=i::LuVolumeX />
        </Show>
      </button>

      <input
        class="theater-playback-controls__volume"
        type="range"
        min="0"
        max="100"
        prop:value=move || volume_value.get()
        on:input=handle_volume_input
        aria-label=move || t_string!(i18n, theater.volume)
        data-testid="theater-volume-slider"
      />

      <button
        type="button"
        class="btn btn--icon"
        on:click=handle_fullscreen
        aria-label=move || {
          if state.is_fullscreen.get() {
            t_string!(i18n, theater.exit_fullscreen).to_string()
          } else {
            t_string!(i18n, theater.fullscreen).to_string()
          }
        }
        data-testid="theater-fullscreen-toggle"
      >
        <Show
          when=move || state.is_fullscreen.get()
          fallback=|| view! { <Icon icon=i::LuMaximize /> }
        >
          <Icon icon=i::LuMinimize />
        </Show>
      </button>
    </div>
  }
}
