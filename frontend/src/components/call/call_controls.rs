//! Bottom control bar for an active call.
//!
//! Renders the mute, camera, screen-share, Picture-in-Picture, and
//! end-call buttons. All handlers delegate to [`crate::call::CallManager`].

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_i18n::t_string;
use wasm_bindgen::JsCast;
use web_sys::HtmlVideoElement;

use crate::call::{use_call_manager, use_call_signals};
use crate::i18n;
use crate::utils::format_duration;

/// Ordered list of CSS selectors used to pick the "best" `<video>`
/// element for Picture-in-Picture (Req 7.3).
///
/// Priority:
///   1. A "hero" tile (typically the active screen-share).
///   2. Any "peer" tile (a non-local participant).
///   3. The "local" tile as a last resort (single-user call).
///
/// Exposed as a `const` array so the priority can be verified by unit
/// tests (round-4 coverage fix).
pub const PIP_VIDEO_SELECTORS: &[&str] = &[
  "[data-pip-candidate='hero'] .video-tile__video",
  "[data-pip-candidate='peer'] .video-tile__video",
  "[data-pip-candidate='local'] .video-tile__video",
];

/// Compute the `aria-pressed` attribute string for a toggle button.
///
/// `pressed=true` → `"true"`, `false` → `"false"`. Exposed as a pure
/// helper so button-state tests do not need a Leptos runtime.
#[must_use]
pub const fn aria_pressed(pressed: bool) -> &'static str {
  if pressed { "true" } else { "false" }
}

/// Render the control bar for the active call.
#[component]
pub fn CallControls() -> impl IntoView {
  let signals = use_call_signals();
  let manager = use_call_manager();
  let i18n = i18n::use_i18n();

  let mic_enabled = Memo::new(move |_| signals.local_media.get().mic_enabled);
  let cam_enabled = Memo::new(move |_| signals.local_media.get().camera_enabled);
  let screen_on = Memo::new(move |_| signals.local_media.get().screen_sharing);
  let pip_on = Memo::new(move |_| signals.pip_active.get());
  let duration = Memo::new(move |_| format_duration(signals.duration_secs.get()));

  let on_mute = {
    let manager = manager.clone();
    move |_| {
      let _ = manager.toggle_mute();
    }
  };

  let on_camera = {
    let manager = manager.clone();
    move |_| {
      let manager = manager.clone();
      spawn_local(async move {
        if let Err(e) = manager.toggle_camera().await {
          web_sys::console::warn_1(&format!("[call] toggle camera failed: {e}").into());
        }
      });
    }
  };

  let on_screen = {
    let manager = manager.clone();
    move |_| {
      let manager = manager.clone();
      spawn_local(async move {
        if let Err(e) = manager.toggle_screen_share().await {
          web_sys::console::warn_1(&format!("[call] screen share failed: {e}").into());
        }
      });
    }
  };

  let on_pip = {
    let manager = manager.clone();
    move |_| {
      let manager = manager.clone();
      let pip_currently_on = pip_on.get_untracked();
      spawn_local(async move {
        if pip_currently_on {
          if let Err(e) = manager.exit_pip().await {
            web_sys::console::warn_1(&format!("[call] exit PiP failed: {e}").into());
          }
          return;
        }
        // P2-New-2 fix: pick the right `<video>` via the
        // `data-pip-candidate` attribute set by `VideoTile`. The
        // selector priority is centralised in [`PIP_VIDEO_SELECTORS`]
        // so it stays resilient to styling changes and is testable.
        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
          return;
        };
        let mut video_el: Option<web_sys::Element> = None;
        for sel in PIP_VIDEO_SELECTORS {
          if let Ok(Some(el)) = document.query_selector(sel) {
            video_el = Some(el);
            break;
          }
        }
        if let Some(el) = video_el.and_then(|e| e.dyn_into::<HtmlVideoElement>().ok())
          && let Err(e) = manager.enter_pip(&el).await
        {
          web_sys::console::warn_1(&format!("[call] PiP failed: {e}").into());
        }
      });
    }
  };

  let on_end = {
    let manager = manager.clone();
    move |_| {
      manager.end_call();
    }
  };

  view! {
    <div class="call-controls" role="toolbar" aria-label=move || t_string!(i18n, call.call)>
      <span class="call-controls__duration" aria-live="polite">
        {move || duration.get()}
      </span>
      <button
        type="button"
        class="call-controls__btn"
        class:is-off=move || !mic_enabled.get()
        on:click=on_mute
        aria-pressed=move || aria_pressed(!mic_enabled.get())
        aria-label=move || {
          if mic_enabled.get() {
            t_string!(i18n, call.mute)
          } else {
            t_string!(i18n, call.unmute)
          }
        }
      >
        {move || if mic_enabled.get() { "🎤" } else { "🔇" }}
      </button>
      <button
        type="button"
        class="call-controls__btn"
        class:is-off=move || !cam_enabled.get()
        on:click=on_camera
        aria-pressed=move || aria_pressed(!cam_enabled.get())
        aria-label=move || t_string!(i18n, call.camera)
      >
        {move || if cam_enabled.get() { "📹" } else { "📷" }}
      </button>
      <button
        type="button"
        class="call-controls__btn"
        class:is-on=move || screen_on.get()
        on:click=on_screen
        aria-pressed=move || aria_pressed(screen_on.get())
        aria-label=move || t_string!(i18n, call.screen_share)
      >
        "🖥"
      </button>
      <button
        type="button"
        class="call-controls__btn"
        class:is-on=move || pip_on.get()
        on:click=on_pip
        aria-pressed=move || aria_pressed(pip_on.get())
        aria-label=move || t_string!(i18n, call.pip)
      >
        "⤢"
      </button>
      <button
        type="button"
        class="call-controls__btn call-controls__btn--danger"
        on:click=on_end
        aria-label=move || t_string!(i18n, call.end)
      >
        "📞"
      </button>
    </div>
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn pip_selector_priority_is_hero_peer_local() {
    assert_eq!(PIP_VIDEO_SELECTORS.len(), 3);
    assert!(PIP_VIDEO_SELECTORS[0].contains("hero"));
    assert!(PIP_VIDEO_SELECTORS[1].contains("peer"));
    assert!(PIP_VIDEO_SELECTORS[2].contains("local"));
  }

  #[test]
  fn pip_selectors_target_video_element() {
    for sel in PIP_VIDEO_SELECTORS {
      assert!(
        sel.ends_with(".video-tile__video"),
        "selector `{sel}` should target the <video> element inside a tile"
      );
    }
  }

  #[test]
  fn aria_pressed_renders_lowercase_strings() {
    assert_eq!(aria_pressed(true), "true");
    assert_eq!(aria_pressed(false), "false");
  }
}
