//! Danmaku overlay canvas (Req 12.5).
//!
//! Rendered as an absolutely-positioned layer on top of the theater
//! video. Consumes two reactive slices of state:
//!
//! * `state.incoming_danmaku` — the queue of freshly-arrived danmaku
//!   pushed by either the local input form or the DataChannel router.
//! * `state.overlay_settings` — visibility, opacity, font-size tier,
//!   and scroll-speed tier chosen by the local viewer.
//!
//! A single periodic tick (`set_interval`) drives two responsibilities:
//!
//! 1. Drain the incoming queue into [`RenderedDanmaku`] entries.
//! 2. Drop expired entries so the DOM doesn't grow unbounded.
//!
//! The canvas never mutates incoming raw danmaku — it only transforms
//! them into render-ready entries.

use js_sys::Date;
use leptos::prelude::*;

use crate::components::theater::DanmakuItem;
use crate::theater::{
  LANE_COUNT, RenderedDanmaku, build_rendered, is_expired, opacity_value, use_theater_state,
};

/// Minimum interval between two canvas maintenance ticks. The CSS
/// keyframe animation drives the actual per-frame motion, so we only
/// need a coarse cadence here for draining / expiration.
const TICK_INTERVAL_MS: u32 = 120;

/// Component entry — the overlay layer.
#[component]
pub fn DanmakuCanvas() -> impl IntoView {
  let state = use_theater_state();

  // The list of on-screen entries. `RwSignal<Vec<_>>` keeps rendering
  // reactive while still being Send+Sync (required by Leptos views).
  let entries = RwSignal::<Vec<RenderedDanmaku>>::new(Vec::new());

  // Monotonic counter for unique render ids.
  let next_id = RwSignal::<u64>::new(1);

  // Per-lane "last assigned timestamp" book-keeping. `RwSignal` is
  // Send+Sync so it can safely cross the interval closure boundary.
  let lane_state = RwSignal::<Vec<u64>>::new(vec![0_u64; LANE_COUNT]);

  // Kick-off tick — runs every `TICK_INTERVAL_MS` until the component
  // is disposed. `set_interval_with_handle` returns a guard that the
  // Leptos runtime drops when the owner goes away.
  let tick_handle = set_interval_with_handle(
    move || {
      let now = Date::now() as u64;

      // Adaptive: skip processing when idle (no entries on screen and
      // no pending arrivals) to avoid wasting CPU cycles.
      let has_entries = entries.with_untracked(|list| !list.is_empty());
      let has_inbound = state.incoming_danmaku.with_untracked(|q| !q.is_empty());
      if !has_entries && !has_inbound {
        return;
      }

      // 1. Drain new arrivals.
      let inbound = state
        .incoming_danmaku
        .try_update(std::mem::take)
        .unwrap_or_default();
      if !inbound.is_empty() {
        let font_tier = state
          .overlay_settings
          .with_untracked(|s| s.danmaku_font_size.clone());
        let speed_tier = state
          .overlay_settings
          .with_untracked(|s| s.danmaku_speed.clone());
        lane_state.update(|lanes| {
          entries.update(|list| {
            for raw in inbound {
              let id = next_id.get_untracked();
              next_id.set(id + 1);
              list.push(build_rendered(
                id,
                &raw,
                &speed_tier,
                &font_tier,
                now,
                lanes,
              ));
            }
          });
        });
      }

      // 2. Evict expired entries.
      entries.update(|list| {
        list.retain(|e| !is_expired(e, now));
      });
    },
    std::time::Duration::from_millis(TICK_INTERVAL_MS.into()),
  );

  // Ensure the timer is cancelled when the component unmounts.
  if let Ok(handle) = tick_handle {
    on_cleanup(move || handle.clear());
  }

  // Layer-level visibility + opacity are driven by overlay settings.
  let layer_style = move || {
    let settings = state.overlay_settings.get();
    let opacity = opacity_value(settings.danmaku_opacity);
    let visibility = if settings.danmaku_visible {
      "visible"
    } else {
      "hidden"
    };
    format!("opacity: {opacity:.2}; visibility: {visibility};")
  };

  view! {
    <div
      class="danmaku-canvas"
      aria-hidden="true"
      data-testid="danmaku-canvas"
      style=layer_style
    >
      <For
        each=move || entries.get()
        key=|entry: &RenderedDanmaku| entry.id
        children=move |entry: RenderedDanmaku| {
          view! { <DanmakuItem entry /> }
        }
      />
    </div>
  }
}
