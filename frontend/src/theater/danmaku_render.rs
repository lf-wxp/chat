//! Danmaku rendering helpers (Req 12.5).
//!
//! Pure presentation logic shared between the input form and the
//! overlay canvas. Nothing here touches `web_sys`, so every branch
//! can be exercised under native unit tests.
//!
//! Pipelined flow for a single danmaku entry:
//!
//! ```text
//!            enqueue      build                 render              retire
//! User ─▶ Danmaku ─▶ RenderedDanmaku ─▶ DanmakuCanvas ─▶ (duration expired)
//! ```

use message::datachannel::Danmaku;
use message::types::DanmakuPosition;

/// Default width (percent) a scrolling danmaku covers when computing
/// the retirement time on slow machines where `requestAnimationFrame`
/// may drop frames. Chosen to match the CSS keyframes — the overlay
/// animation translates from `100%` to `-100%`, i.e. 200% of travel.
pub const SCROLL_TRAVEL_PERCENT: f64 = 200.0;

/// A render-ready danmaku entry with everything the overlay needs to
/// position and style a single DOM node. Lifetimes are tracked in
/// milliseconds so the overlay only needs to subtract against a
/// monotonic clock to decide when to drop the node.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderedDanmaku {
  /// Per-session unique id (monotonically increasing counter).
  pub id: u64,
  /// Text content to render.
  pub text: String,
  /// CSS color string (`#RRGGBB`).
  pub color: String,
  /// Rendered font size in pixels.
  pub font_px: u8,
  /// Display kind — scroll / top / bottom.
  pub kind: DanmakuPosition,
  /// Monotonic creation timestamp (milliseconds).
  pub created_at_ms: u64,
  /// Total on-screen lifetime in milliseconds.
  pub duration_ms: u64,
  /// Track lane (row) index the canvas should place the entry in.
  pub lane: usize,
}

/// Convert a settings tier string into a pixel font-size.
#[must_use]
pub fn font_size_px(tier: &str) -> u8 {
  match tier {
    "small" => 18,
    "large" => 32,
    _ => 24,
  }
}

/// Convert a speed tier string into a total lifetime in milliseconds.
///
/// Slow = 12 s, medium = 8 s, fast = 5 s. Only the scrolling kind
/// uses this directly — pinned (top / bottom) danmaku are fixed at
/// 4 s regardless of the speed preference so they don't block a lane
/// for too long.
#[must_use]
pub fn scroll_duration_ms(tier: &str) -> u64 {
  match tier {
    "slow" => 12_000,
    "fast" => 5_000,
    _ => 8_000,
  }
}

/// Fixed on-screen lifetime of pinned (top / bottom) danmaku.
pub const PINNED_DURATION_MS: u64 = 4_000;

/// Default render lane count. The canvas re-uses lanes when a slot
/// has been free for longer than [`LANE_COOLDOWN_MS`].
pub const LANE_COUNT: usize = 12;

/// Minimum idle time a scrolling lane needs before another entry may
/// share it. Prevents visual overlap on the right edge without needing
/// to measure rendered widths.
pub const LANE_COOLDOWN_MS: u64 = 1_200;

/// Convert a percentage (0–100) into a normalised opacity (0.0–1.0).
#[must_use]
pub fn opacity_value(percent: u8) -> f64 {
  f64::from(percent.min(100)) / 100.0
}

/// Format an RGB integer as a CSS `#RRGGBB` hex string.
#[must_use]
pub fn color_to_css(rgb: u32) -> String {
  format!("#{:06X}", rgb & 0x00FF_FFFF)
}

/// Pick the next free lane. `last_used` tracks the most recent
/// retirement timestamp for each lane; a lane is reusable once
/// `now_ms - last_used[i] >= LANE_COOLDOWN_MS`. Returns the round-
/// robin choice when every lane is still warm.
///
/// The function updates the lane's `last_used` to `now_ms` so the
/// caller does not need to remember which slot was returned.
#[must_use]
pub fn pick_lane(last_used: &mut [u64], now_ms: u64) -> usize {
  if last_used.is_empty() {
    return 0;
  }
  // Prefer the lane that has been idle the longest.
  let mut best = 0_usize;
  let mut best_idle: u64 = 0;
  for (i, ts) in last_used.iter().enumerate() {
    let idle = now_ms.saturating_sub(*ts);
    if idle > best_idle {
      best_idle = idle;
      best = i;
    }
  }
  last_used[best] = now_ms;
  best
}

/// Build a [`RenderedDanmaku`] from a raw [`Danmaku`] payload and the
/// viewer's appearance preferences. Picks a free lane as a side effect
/// on `lane_state`.
#[must_use]
pub fn build_rendered(
  id: u64,
  danmaku: &Danmaku,
  speed_tier: &str,
  font_tier: &str,
  now_ms: u64,
  lane_state: &mut [u64],
) -> RenderedDanmaku {
  let duration_ms = match danmaku.position {
    DanmakuPosition::Scroll => scroll_duration_ms(speed_tier),
    DanmakuPosition::Top | DanmakuPosition::Bottom => PINNED_DURATION_MS,
  };
  let lane = pick_lane(lane_state, now_ms);
  RenderedDanmaku {
    id,
    text: danmaku.content.clone(),
    color: color_to_css(danmaku.color),
    font_px: danmaku.font_size.max(font_size_px(font_tier)),
    kind: danmaku.position,
    created_at_ms: now_ms,
    duration_ms,
    lane,
  }
}

/// Horizontal offset (percent) of a scrolling danmaku at the given
/// elapsed time. Travels from `100%` (right edge, off-screen) to
/// `-100%` (left edge, off-screen).
#[must_use]
pub fn scroll_x_percent(elapsed_ms: u64, duration_ms: u64) -> f64 {
  if duration_ms == 0 {
    return -100.0;
  }
  let progress = (elapsed_ms as f64) / (duration_ms as f64);
  let clamped = progress.clamp(0.0, 1.0);
  100.0 - (clamped * SCROLL_TRAVEL_PERCENT)
}

/// Whether a rendered entry has already lived past its intended
/// lifetime and should be dropped from the canvas.
#[must_use]
pub fn is_expired(entry: &RenderedDanmaku, now_ms: u64) -> bool {
  now_ms.saturating_sub(entry.created_at_ms) >= entry.duration_ms
}

#[cfg(test)]
mod tests {
  use super::*;

  fn make_danmaku(text: &str, pos: DanmakuPosition) -> Danmaku {
    Danmaku {
      content: text.into(),
      font_size: 24,
      color: 0x00FF_FFFF,
      position: pos,
      video_time_ms: 0,
      timestamp_nanos: 0,
    }
  }

  #[test]
  fn font_size_tiers_map_to_expected_pixels() {
    assert_eq!(font_size_px("small"), 18);
    assert_eq!(font_size_px("medium"), 24);
    assert_eq!(font_size_px("large"), 32);
    assert_eq!(font_size_px("unknown"), 24);
  }

  #[test]
  fn scroll_duration_tiers_respect_requirements() {
    assert_eq!(scroll_duration_ms("slow"), 12_000);
    assert_eq!(scroll_duration_ms("medium"), 8_000);
    assert_eq!(scroll_duration_ms("fast"), 5_000);
    assert_eq!(scroll_duration_ms("unknown"), 8_000);
  }

  #[test]
  fn opacity_clamps_above_100_percent() {
    assert!((opacity_value(0) - 0.0).abs() < f64::EPSILON);
    assert!((opacity_value(50) - 0.5).abs() < f64::EPSILON);
    assert!((opacity_value(100) - 1.0).abs() < f64::EPSILON);
    assert!((opacity_value(250) - 1.0).abs() < f64::EPSILON);
  }

  #[test]
  fn color_to_css_strips_high_bits() {
    assert_eq!(color_to_css(0x00FF_FFFF), "#FFFFFF");
    assert_eq!(color_to_css(0xDEAD_BEEF), "#ADBEEF");
    assert_eq!(color_to_css(0), "#000000");
  }

  #[test]
  fn pick_lane_prefers_idle_slot() {
    let mut lanes = [100, 200, 300];
    // lane[0] has been idle the longest (smallest timestamp).
    assert_eq!(pick_lane(&mut lanes, 400), 0);
    assert_eq!(lanes[0], 400);
  }

  #[test]
  fn pick_lane_handles_empty_slice() {
    let mut lanes: [u64; 0] = [];
    assert_eq!(pick_lane(&mut lanes, 0), 0);
  }

  #[test]
  fn scroll_x_percent_maps_progress_linearly() {
    let half = scroll_x_percent(500, 1_000);
    assert!((half - 0.0).abs() < 0.01);
    assert!((scroll_x_percent(0, 1_000) - 100.0).abs() < 0.01);
    assert!((scroll_x_percent(1_000, 1_000) + 100.0).abs() < 0.01);
    // Past duration is clamped at the left edge.
    assert!((scroll_x_percent(2_000, 1_000) + 100.0).abs() < 0.01);
  }

  #[test]
  fn is_expired_uses_creation_plus_duration() {
    let entry = RenderedDanmaku {
      id: 1,
      text: "hi".into(),
      color: "#FFFFFF".into(),
      font_px: 24,
      kind: DanmakuPosition::Scroll,
      created_at_ms: 1_000,
      duration_ms: 5_000,
      lane: 0,
    };
    assert!(!is_expired(&entry, 1_000));
    assert!(!is_expired(&entry, 5_999));
    assert!(is_expired(&entry, 6_000));
    assert!(is_expired(&entry, 10_000));
  }

  #[test]
  fn build_rendered_picks_lane_and_keeps_fields() {
    let mut lanes = [0; 4];
    let dm = make_danmaku("hello", DanmakuPosition::Scroll);
    let entry = build_rendered(7, &dm, "medium", "medium", 5_000, &mut lanes);
    assert_eq!(entry.id, 7);
    assert_eq!(entry.text, "hello");
    assert_eq!(entry.color, "#FFFFFF");
    assert_eq!(entry.duration_ms, 8_000);
    assert_eq!(entry.kind, DanmakuPosition::Scroll);
    assert_eq!(lanes[entry.lane], 5_000);
  }

  #[test]
  fn build_rendered_pins_top_and_bottom_to_4_seconds() {
    let mut lanes = [0; 4];
    let top = make_danmaku("t", DanmakuPosition::Top);
    let bottom = make_danmaku("b", DanmakuPosition::Bottom);
    assert_eq!(
      build_rendered(1, &top, "slow", "medium", 0, &mut lanes).duration_ms,
      PINNED_DURATION_MS,
    );
    assert_eq!(
      build_rendered(2, &bottom, "fast", "medium", 0, &mut lanes).duration_ms,
      PINNED_DURATION_MS,
    );
  }
}
