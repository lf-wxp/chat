//! Native unit tests for `background_section_helpers`.
//!
//! These cover the pure data transforms so regressions in the
//! resize / validation math surface on `cargo test -p chat-frontend
//! --lib --no-default-features` without needing a browser.

use super::*;
use crate::persistence::schema::BACKGROUND_IMAGE_MAX_BYTES;

// ── validate_background_upload ─────────────────────────────────────────

#[test]
fn validate_upload_rejects_empty() {
  assert_eq!(
    validate_background_upload(0, "image/png"),
    Err(UploadRejection::Empty)
  );
}

#[test]
fn validate_upload_rejects_oversize() {
  let over = BACKGROUND_IMAGE_MAX_BYTES + 1;
  assert_eq!(
    validate_background_upload(over, "image/png"),
    Err(UploadRejection::TooLarge { size_bytes: over })
  );
}

#[test]
fn validate_upload_rejects_unsupported_type() {
  assert_eq!(
    validate_background_upload(1024, "image/gif"),
    Err(UploadRejection::UnsupportedType)
  );
  assert_eq!(
    validate_background_upload(1024, "application/pdf"),
    Err(UploadRejection::UnsupportedType)
  );
}

#[test]
fn validate_upload_accepts_canonical_types() {
  for mime in ACCEPTED_IMAGE_MIME_TYPES {
    assert_eq!(
      validate_background_upload(1024, mime),
      Ok(()),
      "expected {mime} to be accepted"
    );
  }
}

#[test]
fn validate_upload_normalises_mime_case_and_whitespace() {
  assert_eq!(validate_background_upload(1024, " IMAGE/PNG "), Ok(()));
  assert_eq!(validate_background_upload(1024, "Image/Webp"), Ok(()));
}

// ── compute_resize_dims ────────────────────────────────────────────────

#[test]
fn resize_returns_input_when_already_within_bounds() {
  assert_eq!(compute_resize_dims(800, 600, 2560, 1440), (800, 600));
  assert_eq!(compute_resize_dims(2560, 1440, 2560, 1440), (2560, 1440));
}

#[test]
fn resize_scales_landscape_by_width() {
  // 5120 × 2880 → cap 2560 → scale 0.5
  let (w, h) = compute_resize_dims(5120, 2880, 2560, 1440);
  assert_eq!((w, h), (2560, 1440));
}

#[test]
fn resize_scales_portrait_by_height() {
  // 1440 × 4320 is narrower but taller than cap — height bottlenecks
  let (w, h) = compute_resize_dims(1440, 4320, 2560, 1440);
  assert_eq!(h, 1440);
  // width scales by the same factor (1/3)
  assert_eq!(w, 480);
}

#[test]
fn resize_preserves_aspect_ratio_within_one_pixel() {
  let (w, h) = compute_resize_dims(3000, 2000, 2560, 1440);
  // aspect ratio 1.5 → ±1 px tolerance after round
  let ratio = f64::from(w) / f64::from(h);
  assert!((ratio - 1.5).abs() < 0.005, "aspect ratio drifted: {ratio}");
}

#[test]
fn resize_handles_zero_dimensions_gracefully() {
  // Degenerate inputs must never cause a divide-by-zero or panic.
  assert_eq!(compute_resize_dims(0, 100, 2560, 1440), (1, 100));
  assert_eq!(compute_resize_dims(100, 0, 2560, 1440), (100, 1));
}

#[test]
fn resize_never_returns_zero_dims_for_tiny_aspect_ratios() {
  // Very thin source (1 × 10000) against 1:1 cap — width rounds down
  // below 1 but must be clamped to 1 so canvas creation can proceed.
  let (w, h) = compute_resize_dims(1, 10_000, 10, 10);
  assert!(w >= 1 && h >= 1);
}

#[test]
fn default_resize_uses_2560_1440_cap() {
  let (w, h) = compute_default_resize_dims(3840, 2160);
  assert!(w <= BACKGROUND_IMAGE_MAX_WIDTH_PX);
  assert!(h <= BACKGROUND_IMAGE_MAX_HEIGHT_PX);
}

// ── slider ↔ value round-trips ─────────────────────────────────────────

#[test]
fn slider_to_blur_px_endpoints() {
  assert_eq!(slider_percent_to_blur_px(0), 0);
  assert_eq!(
    slider_percent_to_blur_px(100),
    crate::settings::BACKGROUND_BLUR_MAX_PX
  );
}

#[test]
fn slider_to_blur_px_clamps_over_100() {
  // Defensive: even if the UI somehow emits 150, the helper caps it.
  assert_eq!(
    slider_percent_to_blur_px(200),
    crate::settings::BACKGROUND_BLUR_MAX_PX
  );
}

#[test]
fn blur_px_to_slider_endpoints() {
  assert_eq!(blur_px_to_slider_percent(0), 0);
  assert_eq!(
    blur_px_to_slider_percent(crate::settings::BACKGROUND_BLUR_MAX_PX),
    100
  );
}

#[test]
fn blur_round_trip_is_idempotent_within_tolerance() {
  // Any slider percent reaches a stable fixed point after one
  // round-trip, which is what the UI depends on to avoid visible
  // drift when opening the panel repeatedly.
  for percent in [0u8, 25, 50, 75, 100] {
    let blur = slider_percent_to_blur_px(percent);
    let back = blur_px_to_slider_percent(blur);
    // Discrete scale: 40 px / 100% → 2.5 px per %, so ±3% tolerance.
    assert!(
      (i32::from(back) - i32::from(percent)).abs() <= 3,
      "round-trip drift too large: {percent} → {blur}px → {back}%"
    );
  }
}

#[test]
fn overlay_slider_endpoints() {
  assert!((slider_percent_to_overlay_alpha(0) - 0.0).abs() < f32::EPSILON);
  assert!(
    (slider_percent_to_overlay_alpha(100) - crate::settings::BACKGROUND_OVERLAY_ALPHA_MAX).abs()
      < f32::EPSILON
  );
}

#[test]
fn overlay_alpha_to_slider_endpoints() {
  assert_eq!(overlay_alpha_to_slider_percent(0.0), 0);
  assert_eq!(
    overlay_alpha_to_slider_percent(crate::settings::BACKGROUND_OVERLAY_ALPHA_MAX),
    100
  );
}

#[test]
fn overlay_round_trip_is_idempotent_within_tolerance() {
  for percent in [0u8, 25, 50, 75, 100] {
    let alpha = slider_percent_to_overlay_alpha(percent);
    let back = overlay_alpha_to_slider_percent(alpha);
    assert!(
      (i32::from(back) - i32::from(percent)).abs() <= 1,
      "round-trip drift too large: {percent}% → {alpha} → {back}%"
    );
  }
}

// ── Gradient Waves sliders (scale / ratio / speed / swell /
// turbulence / tilt / zoom / horizon height / fog depth /
// brightness / opacity) ─────────────────────────────────────────────────
//
// All eleven sliders share the exact same linear-mapping shape (see
// `slider_percent_to_range` / `range_to_slider_percent`), so instead
// of writing 3 near-identical tests per slider (33 total), this
// table-driven test exercises every slider's endpoints and
// round-trip stability in one pass. Each entry pairs a slider's
// `(to_value, to_percent)` functions with its own MIN/MAX constants.
/// One slider's worth of table-driven test wiring: its slider ↔
/// value converter pair, valid range, and a label for assertion
/// failure messages.
type WaveSliderCase = (fn(u8) -> f32, fn(f32) -> u8, f32, f32, &'static str);

#[test]
fn wave_sliders_hit_endpoints_and_round_trip() {
  let cases: &[WaveSliderCase] = &[
    (
      slider_percent_to_wave_scale,
      wave_scale_to_slider_percent,
      crate::settings::BACKGROUND_WAVE_SCALE_MIN,
      crate::settings::BACKGROUND_WAVE_SCALE_MAX,
      "scale",
    ),
    (
      slider_percent_to_wave_ratio,
      wave_ratio_to_slider_percent,
      crate::settings::BACKGROUND_WAVE_RATIO_MIN,
      crate::settings::BACKGROUND_WAVE_RATIO_MAX,
      "ratio",
    ),
    (
      slider_percent_to_wave_speed,
      wave_speed_to_slider_percent,
      crate::settings::BACKGROUND_WAVE_SPEED_MIN,
      crate::settings::BACKGROUND_WAVE_SPEED_MAX,
      "speed",
    ),
    (
      slider_percent_to_wave_swell,
      wave_swell_to_slider_percent,
      crate::settings::BACKGROUND_WAVE_SWELL_MIN,
      crate::settings::BACKGROUND_WAVE_SWELL_MAX,
      "swell",
    ),
    (
      slider_percent_to_wave_turbulence,
      wave_turbulence_to_slider_percent,
      crate::settings::BACKGROUND_WAVE_TURBULENCE_MIN,
      crate::settings::BACKGROUND_WAVE_TURBULENCE_MAX,
      "turbulence",
    ),
    (
      slider_percent_to_wave_tilt,
      wave_tilt_to_slider_percent,
      crate::settings::BACKGROUND_WAVE_TILT_MIN,
      crate::settings::BACKGROUND_WAVE_TILT_MAX,
      "tilt",
    ),
    (
      slider_percent_to_wave_zoom,
      wave_zoom_to_slider_percent,
      crate::settings::BACKGROUND_WAVE_ZOOM_MIN,
      crate::settings::BACKGROUND_WAVE_ZOOM_MAX,
      "zoom",
    ),
    (
      slider_percent_to_wave_horizon_height,
      wave_horizon_height_to_slider_percent,
      crate::settings::BACKGROUND_WAVE_HORIZON_HEIGHT_MIN,
      crate::settings::BACKGROUND_WAVE_HORIZON_HEIGHT_MAX,
      "horizon_height",
    ),
    (
      slider_percent_to_wave_fog_depth,
      wave_fog_depth_to_slider_percent,
      crate::settings::BACKGROUND_WAVE_FOG_DEPTH_MIN,
      crate::settings::BACKGROUND_WAVE_FOG_DEPTH_MAX,
      "fog_depth",
    ),
    (
      slider_percent_to_wave_brightness,
      wave_brightness_to_slider_percent,
      crate::settings::BACKGROUND_WAVE_BRIGHTNESS_MIN,
      crate::settings::BACKGROUND_WAVE_BRIGHTNESS_MAX,
      "brightness",
    ),
    (
      slider_percent_to_wave_opacity,
      wave_opacity_to_slider_percent,
      crate::settings::BACKGROUND_WAVE_OPACITY_MIN,
      crate::settings::BACKGROUND_WAVE_OPACITY_MAX,
      "opacity",
    ),
  ];

  for (to_value, to_percent, min, max, name) in cases {
    // Endpoints.
    assert!(
      (to_value(0) - min).abs() < f32::EPSILON,
      "{name}: 0% should map to MIN"
    );
    assert!(
      (to_value(100) - max).abs() < f32::EPSILON,
      "{name}: 100% should map to MAX"
    );
    assert_eq!(to_percent(*min), 0, "{name}: MIN should map to 0%");
    assert_eq!(to_percent(*max), 100, "{name}: MAX should map to 100%");

    // Round-trip stability.
    for percent in [0u8, 25, 50, 75, 100] {
      let value = to_value(percent);
      let back = to_percent(value);
      assert!(
        (i32::from(back) - i32::from(percent)).abs() <= 1,
        "{name}: round-trip drift too large: {percent}% → {value} → {back}%"
      );
    }
  }
}
