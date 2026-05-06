//! Subtitle overlay rendered on top of the theater video (Req 12.4a).
//!
//! The overlay reads three slices of [`TheaterState`]:
//!
//! * `active_subtitle_text` — the currently-visible cue (refreshed by
//!   the video player's playback effect).
//! * `overlay_settings.subtitle` — user-customised appearance
//!   (position / font size / text color / background opacity).
//! * `subtitle.visible` — whether the track is visible at all.
//!
//! The component is deliberately "dumb" — it contains no scheduling
//! logic. Advancement of the active cue is owned by
//! [`crate::theater::refresh_active_subtitle`] so this overlay can
//! re-render reactively without any local timers.

use leptos::prelude::*;

use crate::theater::{SubtitlePosition, use_theater_state};

/// Subtitle overlay — renders the current cue above or below the
/// video depending on the viewer's preference.
#[component]
pub fn SubtitleOverlay() -> impl IntoView {
  let state = use_theater_state();

  let visible = move || {
    // Hide the overlay when the track itself is toggled off OR when
    // there is no cue currently active.
    let settings_visible = state
      .subtitle
      .with(|t| t.as_ref().is_some_and(|t| t.visible));
    settings_visible && state.active_subtitle_text.with(Option::is_some)
  };

  let container_class = move || {
    let pos = state.overlay_settings.with(|s| s.subtitle.position);
    match pos {
      SubtitlePosition::Top => "subtitle-overlay subtitle-overlay--top",
      SubtitlePosition::Bottom => "subtitle-overlay subtitle-overlay--bottom",
    }
  };

  let line_style = move || {
    let appearance = state.overlay_settings.with(|s| s.subtitle.clone());
    let font_class_size = match appearance.font_size.as_str() {
      "small" => "1rem",
      "large" => "1.75rem",
      _ => "1.25rem",
    };
    // Opacity is 0–80% per state contract; clamp defensively.
    let opacity = f64::from(appearance.background_opacity.min(80)) / 100.0;
    format!(
      "font-size: {font_class_size}; color: {color}; background: rgba(0, 0, 0, {opacity:.2});",
      color = appearance.text_color,
    )
  };

  view! {
    <Show when=visible>
      <div
        class=container_class
        role="region"
        aria-live="polite"
        data-testid="theater-subtitle-overlay"
      >
        <p class="subtitle-overlay__line" style=line_style>
          {move || state.active_subtitle_text.get().unwrap_or_default()}
        </p>
      </div>
    </Show>
  }
}
