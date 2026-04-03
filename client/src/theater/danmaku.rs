//! Danmaku rendering logic
//!
//! Canvas 2D danmaku rendering: scroll/top/bottom danmaku

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

use message::envelope::{Danmaku, DanmakuPosition};

use crate::state::{self, DanmakuItem};

// =============================================================================
// Constants
// =============================================================================

/// Danmaku scroll duration (milliseconds)
pub(super) const DANMAKU_SCROLL_DURATION: f64 = 8000.0;
/// Fixed danmaku display duration (milliseconds)
const DANMAKU_FIXED_DURATION: f64 = 5000.0;
/// Danmaku font size (pixels)
const DANMAKU_FONT_SIZE: f64 = 24.0;
/// Danmaku track height (pixels)
const DANMAKU_TRACK_HEIGHT: f64 = 32.0;
/// Maximum number of danmaku tracks
pub(super) const MAX_TRACKS: u32 = 15;

// =============================================================================
// Danmaku Helper Functions
// =============================================================================

/// Push danmaku into TheaterState
pub fn push_danmaku_to_state(
  theater_state: RwSignal<state::TheaterState>,
  text: &str,
  color: &str,
  position: DanmakuPosition,
  username: &str,
  video_time: f64,
) {
  let now = js_sys::Date::now();
  // Assign track
  let track = theater_state.get_untracked().danmaku_list.len() as u32 % MAX_TRACKS;
  theater_state.update(|s| {
    s.danmaku_list.push(DanmakuItem {
      text: text.to_string(),
      color: color.to_string(),
      position,
      username: username.to_string(),
      created_at: now,
      video_time,
      track,
    });
    // Limit danmaku list length to prevent memory leaks
    if s.danmaku_list.len() > 500 {
      s.danmaku_list.drain(..200);
    }
  });
}

/// Push received Danmaku message into state
pub fn handle_received_danmaku(danmaku: &Danmaku) {
  let theater_state = state::use_theater_state();
  push_danmaku_to_state(
    theater_state,
    &danmaku.text,
    &danmaku.color,
    danmaku.position,
    &danmaku.username,
    danmaku.video_time,
  );
}

// =============================================================================
// Canvas Danmaku Rendering
// =============================================================================

/// Start danmaku Canvas render loop
pub(super) fn start_danmaku_render_loop(
  theater_state: RwSignal<state::TheaterState>,
  show_danmaku: RwSignal<bool>,
) {
  // Use requestAnimationFrame to drive rendering
  let cb = Closure::wrap(Box::new(move || {
    render_danmaku_frame(theater_state, show_danmaku);
  }) as Box<dyn Fn()>);

  if let Some(window) = web_sys::window() {
    let _ = window.request_animation_frame(cb.as_ref().unchecked_ref());
  }
  cb.forget();
}

/// Render single frame of danmaku
fn render_danmaku_frame(
  theater_state: RwSignal<state::TheaterState>,
  show_danmaku: RwSignal<bool>,
) {
  let Some(window) = web_sys::window() else {
    return;
  };
  let Some(document) = window.document() else {
    return;
  };
  let Some(canvas_el) = document.get_element_by_id("danmaku-canvas") else {
    return;
  };
  let canvas: web_sys::HtmlCanvasElement = canvas_el.unchecked_into();

  // Auto-resize Canvas to fit parent container
  let parent = canvas.parent_element();
  if let Some(ref p) = parent {
    let w = p.client_width() as u32;
    let h = p.client_height() as u32;
    if canvas.width() != w || canvas.height() != h {
      canvas.set_width(w);
      canvas.set_height(h);
    }
  }

  let ctx = canvas
    .get_context("2d")
    .ok()
    .flatten()
    .and_then(|c| c.dyn_into::<web_sys::CanvasRenderingContext2d>().ok());
  let Some(ctx) = ctx else { return };

  let cw = canvas.width() as f64;
  let ch = canvas.height() as f64;

  // Clear canvas
  ctx.clear_rect(0.0, 0.0, cw, ch);

  if !show_danmaku.get_untracked() {
    // Danmaku disabled, continue loop but skip rendering
    schedule_next_frame(theater_state, show_danmaku);
    return;
  }

  let now = js_sys::Date::now();
  let font = format!("bold {DANMAKU_FONT_SIZE}px 'Noto Sans SC', sans-serif");
  ctx.set_font(&font);
  ctx.set_text_baseline("top");

  let state = theater_state.get_untracked();

  for item in &state.danmaku_list {
    let age = now - item.created_at;

    match item.position {
      DanmakuPosition::Scroll => {
        // Scroll from right to left
        if age > DANMAKU_SCROLL_DURATION {
          continue; // Expired
        }
        let progress = age / DANMAKU_SCROLL_DURATION;
        let text_width = ctx
          .measure_text(&item.text)
          .map(|m| m.width())
          .unwrap_or(100.0);
        let x = cw - progress * (cw + text_width);
        let y = (item.track as f64) * DANMAKU_TRACK_HEIGHT + 4.0;

        // Draw shadow
        ctx.set_shadow_color("rgba(0,0,0,0.6)");
        ctx.set_shadow_blur(2.0);
        ctx.set_shadow_offset_x(1.0);
        ctx.set_shadow_offset_y(1.0);

        ctx.set_fill_style_str(&item.color);
        ctx.fill_text(&item.text, x, y).ok();

        // Reset shadow
        ctx.set_shadow_color("transparent");
        ctx.set_shadow_blur(0.0);
        ctx.set_shadow_offset_x(0.0);
        ctx.set_shadow_offset_y(0.0);
      }
      DanmakuPosition::Top => {
        if age > DANMAKU_FIXED_DURATION {
          continue;
        }
        let text_width = ctx
          .measure_text(&item.text)
          .map(|m| m.width())
          .unwrap_or(100.0);
        let x = (cw - text_width) / 2.0;
        let y = (item.track as f64 % 5.0) * DANMAKU_TRACK_HEIGHT + 4.0;

        ctx.set_shadow_color("rgba(0,0,0,0.6)");
        ctx.set_shadow_blur(2.0);
        ctx.set_fill_style_str(&item.color);
        ctx.fill_text(&item.text, x, y).ok();
        ctx.set_shadow_color("transparent");
        ctx.set_shadow_blur(0.0);
      }
      DanmakuPosition::Bottom => {
        if age > DANMAKU_FIXED_DURATION {
          continue;
        }
        let text_width = ctx
          .measure_text(&item.text)
          .map(|m| m.width())
          .unwrap_or(100.0);
        let x = (cw - text_width) / 2.0;
        let y = ch - ((item.track as f64 % 5.0) + 1.0) * DANMAKU_TRACK_HEIGHT;

        ctx.set_shadow_color("rgba(0,0,0,0.6)");
        ctx.set_shadow_blur(2.0);
        ctx.set_fill_style_str(&item.color);
        ctx.fill_text(&item.text, x, y).ok();
        ctx.set_shadow_color("transparent");
        ctx.set_shadow_blur(0.0);
      }
    }
  }

  // Continue to next frame
  schedule_next_frame(theater_state, show_danmaku);
}

/// Schedule next frame rendering
fn schedule_next_frame(theater_state: RwSignal<state::TheaterState>, show_danmaku: RwSignal<bool>) {
  let cb = Closure::wrap(Box::new(move || {
    render_danmaku_frame(theater_state, show_danmaku);
  }) as Box<dyn Fn()>);

  if let Some(window) = web_sys::window() {
    let _ = window.request_animation_frame(cb.as_ref().unchecked_ref());
  }
  cb.forget();
}
