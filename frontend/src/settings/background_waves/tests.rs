//! Unit tests for [`super::WaveConfig`].

use super::*;

#[test]
fn default_matches_original_shader_constants() {
  let c = WaveConfig::default();
  assert!((c.scale - BACKGROUND_WAVE_SCALE_DEFAULT).abs() < f32::EPSILON);
  assert!((c.ratio - BACKGROUND_WAVE_RATIO_DEFAULT).abs() < f32::EPSILON);
  assert!((c.speed - 1.0).abs() < f32::EPSILON);
  assert!((c.swell - 1.0).abs() < f32::EPSILON);
  assert!((c.turbulence - 1.0).abs() < f32::EPSILON);
  assert!((c.tilt - 1.11).abs() < f32::EPSILON);
  assert!((c.zoom - 1.0).abs() < f32::EPSILON);
  assert!((c.horizon_height - 0.0).abs() < f32::EPSILON);
  assert!((c.fog_depth - 32.0).abs() < f32::EPSILON);
  assert!((c.brightness - 1.0).abs() < f32::EPSILON);
  assert!((c.opacity - 1.0).abs() < f32::EPSILON);
}

#[test]
fn default_is_already_sanitised() {
  let c = WaveConfig::default();
  assert_eq!(c, c.sanitised());
}

/// One field's worth of table-driven clamp coverage: a setter for
/// that field plus its valid MIN/MAX bounds.
type WaveFieldCase = (fn(&mut WaveConfig, f32), f32, f32);

/// Table-driven clamp coverage: for each field, blow it way over/
/// under its valid range and check `sanitised()` pins it to the
/// exact bound. Written as one test (rather than 11 near-identical
/// tests) since each case is only meaningfully different in which
/// field + which constants it touches.
#[test]
fn sanitised_clamps_every_field_to_its_range() {
  let cases: &[WaveFieldCase] = &[
    (
      |c, v| c.scale = v,
      BACKGROUND_WAVE_SCALE_MIN,
      BACKGROUND_WAVE_SCALE_MAX,
    ),
    (
      |c, v| c.ratio = v,
      BACKGROUND_WAVE_RATIO_MIN,
      BACKGROUND_WAVE_RATIO_MAX,
    ),
    (
      |c, v| c.speed = v,
      BACKGROUND_WAVE_SPEED_MIN,
      BACKGROUND_WAVE_SPEED_MAX,
    ),
    (
      |c, v| c.swell = v,
      BACKGROUND_WAVE_SWELL_MIN,
      BACKGROUND_WAVE_SWELL_MAX,
    ),
    (
      |c, v| c.turbulence = v,
      BACKGROUND_WAVE_TURBULENCE_MIN,
      BACKGROUND_WAVE_TURBULENCE_MAX,
    ),
    (
      |c, v| c.tilt = v,
      BACKGROUND_WAVE_TILT_MIN,
      BACKGROUND_WAVE_TILT_MAX,
    ),
    (
      |c, v| c.zoom = v,
      BACKGROUND_WAVE_ZOOM_MIN,
      BACKGROUND_WAVE_ZOOM_MAX,
    ),
    (
      |c, v| c.horizon_height = v,
      BACKGROUND_WAVE_HORIZON_HEIGHT_MIN,
      BACKGROUND_WAVE_HORIZON_HEIGHT_MAX,
    ),
    (
      |c, v| c.fog_depth = v,
      BACKGROUND_WAVE_FOG_DEPTH_MIN,
      BACKGROUND_WAVE_FOG_DEPTH_MAX,
    ),
    (
      |c, v| c.brightness = v,
      BACKGROUND_WAVE_BRIGHTNESS_MIN,
      BACKGROUND_WAVE_BRIGHTNESS_MAX,
    ),
    (
      |c, v| c.opacity = v,
      BACKGROUND_WAVE_OPACITY_MIN,
      BACKGROUND_WAVE_OPACITY_MAX,
    ),
  ];

  for (setter, min, max) in cases {
    let mut over = WaveConfig::default();
    setter(&mut over, max + 1000.0);
    let sanitised_over = over.sanitised();

    let mut under = WaveConfig::default();
    setter(&mut under, min - 1000.0);
    let sanitised_under = under.sanitised();

    // Extract the field back out by re-running the setter against a
    // zeroed sentinel and comparing — simplest is just to re-check
    // via the same closure's effect: since `sanitised` returns the
    // whole struct, compare against a struct built by applying the
    // bound directly.
    let mut expect_max = WaveConfig::default();
    setter(&mut expect_max, *max);
    let mut expect_min = WaveConfig::default();
    setter(&mut expect_min, *min);

    assert_eq!(
      sanitised_over, expect_max,
      "expected clamp to MAX={max} after overshoot"
    );
    assert_eq!(
      sanitised_under, expect_min,
      "expected clamp to MIN={min} after undershoot"
    );
  }
}
