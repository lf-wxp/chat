//! Single danmaku node rendered inside [`DanmakuCanvas`].
//!
//! Stateless — it derives every CSS declaration from the immutable
//! [`RenderedDanmaku`] prop. Scrolling entries rely on the CSS
//! keyframes declared in `theater.css`; pinned (top / bottom) entries
//! are centered horizontally with a font-size-derived offset so multi-
//! line stacks don't overlap.

use leptos::prelude::*;
use message::types::DanmakuPosition;

use crate::theater::RenderedDanmaku;

/// Render a single danmaku entry.
#[component]
pub fn DanmakuItem(entry: RenderedDanmaku) -> impl IntoView {
  let RenderedDanmaku {
    color,
    font_px,
    kind,
    duration_ms,
    lane,
    text,
    ..
  } = entry;

  let lane_offset_px = u64::from(font_px) * (u64::try_from(lane).unwrap_or(0) + 1);
  let style = match kind {
    DanmakuPosition::Scroll => format!(
      "color: {color}; font-size: {font_px}px; top: {top}px; \
       animation-duration: {dur}ms; animation-timing-function: linear;",
      top = lane_offset_px,
      dur = duration_ms,
    ),
    DanmakuPosition::Top => format!(
      "color: {color}; font-size: {font_px}px; top: {top}px; \
       left: 50%; transform: translateX(-50%);",
      top = lane_offset_px,
    ),
    DanmakuPosition::Bottom => format!(
      "color: {color}; font-size: {font_px}px; bottom: {bottom}px; \
       left: 50%; transform: translateX(-50%);",
      bottom = lane_offset_px,
    ),
  };

  let class = match kind {
    DanmakuPosition::Scroll => "danmaku-canvas__entry danmaku-canvas__entry--scroll",
    DanmakuPosition::Top => "danmaku-canvas__entry danmaku-canvas__entry--top",
    DanmakuPosition::Bottom => "danmaku-canvas__entry danmaku-canvas__entry--bottom",
  };

  view! {
    <span class=class style=style>
      {text}
    </span>
  }
}
