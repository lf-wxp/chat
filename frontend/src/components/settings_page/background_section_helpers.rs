//! Pure helpers backing the `BackgroundSection` settings UI.
//!
//! None of the functions here touch the DOM or IndexedDB — they are
//! deliberately kept as plain Rust so the unit tests (see
//! `background_section_helpers/tests.rs`) can run on the native
//! target without a browser. The UI layer composes these helpers
//! with the async IDB CRUD in `persistence::store::background_image`.
//!
//! Individual items are gated on `target_arch = "wasm32"` plus the
//! `test` cfg so native `cargo check` on non-test targets does not
//! flag them as dead_code. The module itself stays visible so any
//! future native-visible helper can be added without a cfg dance.

use crate::persistence::schema::BACKGROUND_IMAGE_MAX_BYTES;

/// Upper bound on the width or height of an uploaded background
/// image after client-side downscaling. Anything larger is shrunk
/// proportionally so the stored blob stays inside
/// [`BACKGROUND_IMAGE_MAX_BYTES`] even for busy photographs.
#[cfg(any(target_arch = "wasm32", test))]
pub const BACKGROUND_IMAGE_MAX_WIDTH_PX: u32 = 2560;
/// Companion limit for height. 16:9 friendly so 4K → QHD landscape.
#[cfg(any(target_arch = "wasm32", test))]
pub const BACKGROUND_IMAGE_MAX_HEIGHT_PX: u32 = 1440;

/// Canonical list of image MIME types accepted by the upload flow.
/// JPEG / PNG / WebP / AVIF cover the formats the browser can
/// re-encode into WebP via `<canvas>.toBlob`. `image/gif` is
/// intentionally excluded because animated frames would be stripped
/// by the canvas round-trip — surfacing that as an error is clearer
/// than silently losing the animation.
pub const ACCEPTED_IMAGE_MIME_TYPES: &[&str] = &[
  "image/jpeg",
  "image/jpg",
  "image/png",
  "image/webp",
  "image/avif",
];

/// Reasons an upload can be rejected before ever touching
/// IndexedDB. Surfaced verbatim to the UI so callers can map each
/// variant to a localised toast message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadRejection {
  /// The blob reports zero bytes — either the file was empty or the
  /// `File` object was revoked before we read it.
  Empty,
  /// The blob exceeds [`BACKGROUND_IMAGE_MAX_BYTES`]. Pre-compression
  /// check: even after canvas re-encoding, there is no guarantee the
  /// output stays under the cap for pathological inputs.
  TooLarge { size_bytes: u64 },
  /// The MIME type is not in [`ACCEPTED_IMAGE_MIME_TYPES`].
  UnsupportedType,
}

/// Validate the basic shape of an upload before doing any expensive
/// decode / compression work. Returns `Ok(())` when the blob is
/// acceptable, otherwise a categorised rejection that the UI layer
/// can translate into a user-facing message.
///
/// `size_bytes` must be supplied by the caller (the `Blob` API's
/// `size` getter works in wasm but we keep the signature plain-Rust
/// so tests don't need a browser).
pub fn validate_background_upload(size_bytes: u64, mime: &str) -> Result<(), UploadRejection> {
  if size_bytes == 0 {
    return Err(UploadRejection::Empty);
  }
  if size_bytes > BACKGROUND_IMAGE_MAX_BYTES {
    return Err(UploadRejection::TooLarge { size_bytes });
  }
  let normalised = mime.trim().to_ascii_lowercase();
  if !ACCEPTED_IMAGE_MIME_TYPES.contains(&normalised.as_str()) {
    return Err(UploadRejection::UnsupportedType);
  }
  Ok(())
}

/// Compute the target `(width, height)` for a canvas downscale while
/// preserving the source aspect ratio. When the source already fits
/// inside `(max_w, max_h)`, the pair is returned unchanged so we
/// never blow up a small 800×600 upload to 2560×1920.
///
/// Uses `u64` internally to avoid overflow on very large sources
/// (e.g. 20 000 × 15 000 scanner output) even though the output is
/// narrowed back to `u32`.
#[cfg(any(target_arch = "wasm32", test))]
#[must_use]
pub fn compute_resize_dims(width: u32, height: u32, max_w: u32, max_h: u32) -> (u32, u32) {
  if width == 0 || height == 0 {
    return (width.max(1), height.max(1));
  }
  if width <= max_w && height <= max_h {
    return (width, height);
  }
  // Scale by whichever axis overflows the most. Computing the ratio
  // as an f64 is fine for UI-scale dimensions; no risk of losing
  // precision for values ≤ 1e6.
  let w = f64::from(width);
  let h = f64::from(height);
  let scale = (f64::from(max_w) / w).min(f64::from(max_h) / h);
  // `scale` is strictly < 1.0 here because at least one axis
  // overflows; rounding to at least 1 prevents zero-sized canvases.
  let new_w = ((w * scale).round() as u32).max(1);
  let new_h = ((h * scale).round() as u32).max(1);
  (new_w, new_h)
}

/// Convenience wrapper that plugs [`BACKGROUND_IMAGE_MAX_WIDTH_PX`]
/// and [`BACKGROUND_IMAGE_MAX_HEIGHT_PX`] into
/// [`compute_resize_dims`] so callers don't have to repeat the
/// constants.
#[cfg(any(target_arch = "wasm32", test))]
#[must_use]
pub fn compute_default_resize_dims(width: u32, height: u32) -> (u32, u32) {
  compute_resize_dims(
    width,
    height,
    BACKGROUND_IMAGE_MAX_WIDTH_PX,
    BACKGROUND_IMAGE_MAX_HEIGHT_PX,
  )
}

/// Clamp a user-supplied blur slider value (percentage in 0..=100)
/// into the persisted `u8` blur pixel range
/// (`0..=BACKGROUND_BLUR_MAX_PX`). The UI uses a 0–100 slider so the
/// mapping is a simple linear scale to 0..=40 px.
#[must_use]
pub fn slider_percent_to_blur_px(percent: u8) -> u8 {
  let clamped = percent.min(100);
  // 100% maps to BACKGROUND_BLUR_MAX_PX (40 px).
  let scaled = (u32::from(clamped) * u32::from(crate::settings::BACKGROUND_BLUR_MAX_PX) + 50) / 100;
  scaled as u8
}

/// Reverse of [`slider_percent_to_blur_px`] — used to initialise the
/// slider from persisted state without losing idempotency.
#[must_use]
pub fn blur_px_to_slider_percent(blur_px: u8) -> u8 {
  let capped = blur_px.min(crate::settings::BACKGROUND_BLUR_MAX_PX);
  ((u32::from(capped) * 100) / u32::from(crate::settings::BACKGROUND_BLUR_MAX_PX)) as u8
}

/// Convert a 0–100 overlay slider value to the persisted
/// `overlay_alpha` in `0.0..=BACKGROUND_OVERLAY_ALPHA_MAX`.
#[must_use]
pub fn slider_percent_to_overlay_alpha(percent: u8) -> f32 {
  let clamped = f32::from(percent.min(100));
  (clamped / 100.0) * crate::settings::BACKGROUND_OVERLAY_ALPHA_MAX
}

/// Reverse of [`slider_percent_to_overlay_alpha`].
#[must_use]
pub fn overlay_alpha_to_slider_percent(alpha: f32) -> u8 {
  let capped = alpha.clamp(0.0, crate::settings::BACKGROUND_OVERLAY_ALPHA_MAX);
  ((capped / crate::settings::BACKGROUND_OVERLAY_ALPHA_MAX) * 100.0).round() as u8
}

/// Convert a 0–100 slider percentage to a value linearly interpolated
/// across `[min, max]`. Shared by every Gradient Waves slider, which
/// (unlike blur/overlay) don't all start at zero.
#[must_use]
pub fn slider_percent_to_range(percent: u8, min: f32, max: f32) -> f32 {
  let clamped = f32::from(percent.min(100)) / 100.0;
  min + clamped * (max - min)
}

/// Reverse of [`slider_percent_to_range`].
#[must_use]
pub fn range_to_slider_percent(value: f32, min: f32, max: f32) -> u8 {
  if max <= min {
    return 0;
  }
  let capped = value.clamp(min, max);
  (((capped - min) / (max - min)) * 100.0).round() as u8
}

/// Generates a `slider_percent_to_wave_*` / `wave_*_to_slider_percent`
/// pair for a [`crate::settings::WaveConfig`]
/// field, wired to the field's `BACKGROUND_WAVE_*_MIN`/`_MAX`
/// constants. Keeps the eleven near-identical conversions (one per
/// slider) from turning into 44 lines of copy-pasted boilerplate
/// that would drift out of sync with the constants.
macro_rules! wave_slider_pair {
  ($to_value:ident, $to_percent:ident, $min:expr, $max:expr) => {
    // Slider percentage (0-100) -> persisted value in `$min..=$max`.
    #[must_use]
    pub fn $to_value(percent: u8) -> f32 {
      slider_percent_to_range(percent, $min, $max)
    }

    // Reverse of `$to_value` above.
    #[must_use]
    pub fn $to_percent(value: f32) -> u8 {
      range_to_slider_percent(value, $min, $max)
    }
  };
}

wave_slider_pair!(
  slider_percent_to_wave_scale,
  wave_scale_to_slider_percent,
  crate::settings::BACKGROUND_WAVE_SCALE_MIN,
  crate::settings::BACKGROUND_WAVE_SCALE_MAX
);
wave_slider_pair!(
  slider_percent_to_wave_ratio,
  wave_ratio_to_slider_percent,
  crate::settings::BACKGROUND_WAVE_RATIO_MIN,
  crate::settings::BACKGROUND_WAVE_RATIO_MAX
);
wave_slider_pair!(
  slider_percent_to_wave_speed,
  wave_speed_to_slider_percent,
  crate::settings::BACKGROUND_WAVE_SPEED_MIN,
  crate::settings::BACKGROUND_WAVE_SPEED_MAX
);
wave_slider_pair!(
  slider_percent_to_wave_swell,
  wave_swell_to_slider_percent,
  crate::settings::BACKGROUND_WAVE_SWELL_MIN,
  crate::settings::BACKGROUND_WAVE_SWELL_MAX
);
wave_slider_pair!(
  slider_percent_to_wave_turbulence,
  wave_turbulence_to_slider_percent,
  crate::settings::BACKGROUND_WAVE_TURBULENCE_MIN,
  crate::settings::BACKGROUND_WAVE_TURBULENCE_MAX
);
wave_slider_pair!(
  slider_percent_to_wave_tilt,
  wave_tilt_to_slider_percent,
  crate::settings::BACKGROUND_WAVE_TILT_MIN,
  crate::settings::BACKGROUND_WAVE_TILT_MAX
);
wave_slider_pair!(
  slider_percent_to_wave_zoom,
  wave_zoom_to_slider_percent,
  crate::settings::BACKGROUND_WAVE_ZOOM_MIN,
  crate::settings::BACKGROUND_WAVE_ZOOM_MAX
);
wave_slider_pair!(
  slider_percent_to_wave_horizon_height,
  wave_horizon_height_to_slider_percent,
  crate::settings::BACKGROUND_WAVE_HORIZON_HEIGHT_MIN,
  crate::settings::BACKGROUND_WAVE_HORIZON_HEIGHT_MAX
);
wave_slider_pair!(
  slider_percent_to_wave_fog_depth,
  wave_fog_depth_to_slider_percent,
  crate::settings::BACKGROUND_WAVE_FOG_DEPTH_MIN,
  crate::settings::BACKGROUND_WAVE_FOG_DEPTH_MAX
);
wave_slider_pair!(
  slider_percent_to_wave_brightness,
  wave_brightness_to_slider_percent,
  crate::settings::BACKGROUND_WAVE_BRIGHTNESS_MIN,
  crate::settings::BACKGROUND_WAVE_BRIGHTNESS_MAX
);
wave_slider_pair!(
  slider_percent_to_wave_opacity,
  wave_opacity_to_slider_percent,
  crate::settings::BACKGROUND_WAVE_OPACITY_MIN,
  crate::settings::BACKGROUND_WAVE_OPACITY_MAX
);

#[cfg(test)]
mod tests;
