//! Gradient Waves shader configuration.
//!
//! [`WaveConfig`] carries every user-tunable knob for the raymarched
//! plasma heightfield rendered by `components::webgl_background`
//! (see that module's `FRAG_WAVES` shader for how each field maps
//! onto the GLSL uniforms). Kept in its own module — rather than
//! inlined into `types::BackgroundSettings` — because eleven
//! independent numeric ranges each need their own MIN/MAX/DEFAULT
//! trio, and bundling all of that into the already-large
//! `types.rs` would make the file hard to scan.

use serde::{Deserialize, Serialize};

// ── Spatial frequency ───────────────────────────────────────────────────

/// Valid range for the "scale" knob — the plasma's overall spatial
/// frequency multiplier. Below the minimum the waves flatten into a
/// barely-undulating sheet; above the maximum they fragment into
/// visual noise.
pub const BACKGROUND_WAVE_SCALE_MIN: f32 = 0.3;
pub const BACKGROUND_WAVE_SCALE_MAX: f32 = 2.5;
/// Matches the original hard-coded shader constant (`0.6` inside the
/// `freq` vector) once normalised to this knob's `1.0 == unchanged`
/// convention.
pub const BACKGROUND_WAVE_SCALE_DEFAULT: f32 = 1.0;

/// Valid range for the "ratio" knob — the aspect ratio between the
/// wave pattern's x/y spatial frequencies. Low values read as long
/// horizontal swells; high values read as tight vertical ripples.
pub const BACKGROUND_WAVE_RATIO_MIN: f32 = 0.5;
pub const BACKGROUND_WAVE_RATIO_MAX: f32 = 4.0;
/// Matches the original hard-coded shader constant (`7.0 / 3.0 ≈
/// 2.33`).
pub const BACKGROUND_WAVE_RATIO_DEFAULT: f32 = 7.0 / 3.0;

// ── Motion ───────────────────────────────────────────────────────────────

/// Valid range for the "speed" knob — multiplies the shader's time
/// input, so `2.0` plays the animation twice as fast.
pub const BACKGROUND_WAVE_SPEED_MIN: f32 = 0.1;
pub const BACKGROUND_WAVE_SPEED_MAX: f32 = 3.0;
/// Matches the original hard-coded `iTime * 0.4` playback rate.
pub const BACKGROUND_WAVE_SPEED_DEFAULT: f32 = 1.0;

// ── Terrain shape ────────────────────────────────────────────────────────

/// Valid range for the "swell" knob — amplitude multiplier on the
/// terrain's up/down undulation. `0.2` reads as near-flat; `3.0`
/// produces towering peaks and troughs.
pub const BACKGROUND_WAVE_SWELL_MIN: f32 = 0.2;
pub const BACKGROUND_WAVE_SWELL_MAX: f32 = 3.0;
/// Matches the original hard-coded `2.5` amplitude terms.
pub const BACKGROUND_WAVE_SWELL_DEFAULT: f32 = 1.0;

/// Valid range for the "turbulence" knob — magnitude multiplier on
/// the plasma's domain-warp terms (the `35.0 * sin(...)` /
/// `20.0 * cos(...)` offsets that make the ripples look organic
/// rather than perfectly sinusoidal). `0.0` disables the warp
/// entirely for clean, regular waves.
pub const BACKGROUND_WAVE_TURBULENCE_MIN: f32 = 0.0;
pub const BACKGROUND_WAVE_TURBULENCE_MAX: f32 = 3.0;
/// Matches the original hard-coded warp magnitude.
pub const BACKGROUND_WAVE_TURBULENCE_DEFAULT: f32 = 1.0;

/// Valid range for the "horizon height" knob — additive offset on
/// the terrain's baseline depth (the hard-coded `5.5` constant in
/// the original shader). Positive values push the terrain farther
/// away, revealing more "sky"/horizon color; negative values bring
/// it closer.
pub const BACKGROUND_WAVE_HORIZON_HEIGHT_MIN: f32 = -3.0;
pub const BACKGROUND_WAVE_HORIZON_HEIGHT_MAX: f32 = 3.0;
pub const BACKGROUND_WAVE_HORIZON_HEIGHT_DEFAULT: f32 = 0.0;

// ── Camera ───────────────────────────────────────────────────────────────

/// Valid range for the "tilt" knob — the camera's fixed pitch angle
/// (radians) looking down at the terrain plane. Lower values look
/// nearly top-down; higher values look nearly level with the
/// horizon.
pub const BACKGROUND_WAVE_TILT_MIN: f32 = 0.3;
pub const BACKGROUND_WAVE_TILT_MAX: f32 = 2.2;
/// Matches the original hard-coded `1.11` radian tilt.
pub const BACKGROUND_WAVE_TILT_DEFAULT: f32 = 1.11;

/// Valid range for the "zoom" knob — divides the camera's
/// field-of-view, so values above `1.0` zoom in (narrower FOV,
/// closer-looking terrain) and values below `1.0` zoom out.
pub const BACKGROUND_WAVE_ZOOM_MIN: f32 = 0.4;
pub const BACKGROUND_WAVE_ZOOM_MAX: f32 = 2.5;
pub const BACKGROUND_WAVE_ZOOM_DEFAULT: f32 = 1.0;

// ── Depth / color ────────────────────────────────────────────────────────

/// Valid range for the "fog depth" knob — the raymarch reference
/// distance (world units) at which the terrain fully fades into the
/// horizon color. Smaller values fog out sooner (terrain reads as
/// closer/denser fog); larger values let detail reach farther before
/// fading.
pub const BACKGROUND_WAVE_FOG_DEPTH_MIN: f32 = 10.0;
pub const BACKGROUND_WAVE_FOG_DEPTH_MAX: f32 = 80.0;
/// Matches the original hard-coded `32.0` reference distance.
pub const BACKGROUND_WAVE_FOG_DEPTH_DEFAULT: f32 = 32.0;

/// Valid range for the "brightness" knob — multiplies the final
/// terrain/horizon color before alpha compositing.
pub const BACKGROUND_WAVE_BRIGHTNESS_MIN: f32 = 0.3;
pub const BACKGROUND_WAVE_BRIGHTNESS_MAX: f32 = 2.0;
pub const BACKGROUND_WAVE_BRIGHTNESS_DEFAULT: f32 = 1.0;

/// Valid range for the "opacity" knob — multiplies the theme
/// palette's base waves opacity, letting the user push the effect
/// stronger or fainter than the theme default without having to
/// pick a whole different theme.
pub const BACKGROUND_WAVE_OPACITY_MIN: f32 = 0.0;
pub const BACKGROUND_WAVE_OPACITY_MAX: f32 = 2.0;
pub const BACKGROUND_WAVE_OPACITY_DEFAULT: f32 = 1.0;

/// All user-tunable Gradient Waves parameters, threaded straight
/// through to the `FRAG_WAVES` shader's uniforms every frame. Every
/// field is a plain multiplier/offset around the original
/// hard-coded shader constants, so [`WaveConfig::default`]
/// reproduces the pre-configurable look exactly.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WaveConfig {
  /// Spatial frequency multiplier. See
  /// [`BACKGROUND_WAVE_SCALE_MIN`]/[`BACKGROUND_WAVE_SCALE_MAX`].
  pub scale: f32,
  /// x:y frequency ratio. See
  /// [`BACKGROUND_WAVE_RATIO_MIN`]/[`BACKGROUND_WAVE_RATIO_MAX`].
  pub ratio: f32,
  /// Animation playback speed multiplier. See
  /// [`BACKGROUND_WAVE_SPEED_MIN`]/[`BACKGROUND_WAVE_SPEED_MAX`].
  pub speed: f32,
  /// Terrain undulation amplitude multiplier. See
  /// [`BACKGROUND_WAVE_SWELL_MIN`]/[`BACKGROUND_WAVE_SWELL_MAX`].
  pub swell: f32,
  /// Domain-warp magnitude multiplier. See
  /// [`BACKGROUND_WAVE_TURBULENCE_MIN`]/[`BACKGROUND_WAVE_TURBULENCE_MAX`].
  pub turbulence: f32,
  /// Camera pitch angle in radians. See
  /// [`BACKGROUND_WAVE_TILT_MIN`]/[`BACKGROUND_WAVE_TILT_MAX`].
  pub tilt: f32,
  /// Field-of-view zoom factor. See
  /// [`BACKGROUND_WAVE_ZOOM_MIN`]/[`BACKGROUND_WAVE_ZOOM_MAX`].
  pub zoom: f32,
  /// Terrain baseline depth offset. See
  /// [`BACKGROUND_WAVE_HORIZON_HEIGHT_MIN`]/[`BACKGROUND_WAVE_HORIZON_HEIGHT_MAX`].
  pub horizon_height: f32,
  /// Raymarch fog reference distance (world units). See
  /// [`BACKGROUND_WAVE_FOG_DEPTH_MIN`]/[`BACKGROUND_WAVE_FOG_DEPTH_MAX`].
  pub fog_depth: f32,
  /// Final color brightness multiplier. See
  /// [`BACKGROUND_WAVE_BRIGHTNESS_MIN`]/[`BACKGROUND_WAVE_BRIGHTNESS_MAX`].
  pub brightness: f32,
  /// Opacity multiplier applied on top of the theme palette's base
  /// waves opacity. See
  /// [`BACKGROUND_WAVE_OPACITY_MIN`]/[`BACKGROUND_WAVE_OPACITY_MAX`].
  pub opacity: f32,
}

impl Default for WaveConfig {
  fn default() -> Self {
    Self {
      scale: BACKGROUND_WAVE_SCALE_DEFAULT,
      ratio: BACKGROUND_WAVE_RATIO_DEFAULT,
      speed: BACKGROUND_WAVE_SPEED_DEFAULT,
      swell: BACKGROUND_WAVE_SWELL_DEFAULT,
      turbulence: BACKGROUND_WAVE_TURBULENCE_DEFAULT,
      tilt: BACKGROUND_WAVE_TILT_DEFAULT,
      zoom: BACKGROUND_WAVE_ZOOM_DEFAULT,
      horizon_height: BACKGROUND_WAVE_HORIZON_HEIGHT_DEFAULT,
      fog_depth: BACKGROUND_WAVE_FOG_DEPTH_DEFAULT,
      brightness: BACKGROUND_WAVE_BRIGHTNESS_DEFAULT,
      opacity: BACKGROUND_WAVE_OPACITY_DEFAULT,
    }
  }
}

impl WaveConfig {
  /// Clamp every field into its valid range. Called from
  /// [`super::types::BackgroundSettings::sanitised`] so a
  /// hand-edited localStorage value cannot push the shader into a
  /// degenerate (NaN/Inf-producing or all-black) state.
  #[must_use]
  pub fn sanitised(mut self) -> Self {
    self.scale = self
      .scale
      .clamp(BACKGROUND_WAVE_SCALE_MIN, BACKGROUND_WAVE_SCALE_MAX);
    self.ratio = self
      .ratio
      .clamp(BACKGROUND_WAVE_RATIO_MIN, BACKGROUND_WAVE_RATIO_MAX);
    self.speed = self
      .speed
      .clamp(BACKGROUND_WAVE_SPEED_MIN, BACKGROUND_WAVE_SPEED_MAX);
    self.swell = self
      .swell
      .clamp(BACKGROUND_WAVE_SWELL_MIN, BACKGROUND_WAVE_SWELL_MAX);
    self.turbulence = self.turbulence.clamp(
      BACKGROUND_WAVE_TURBULENCE_MIN,
      BACKGROUND_WAVE_TURBULENCE_MAX,
    );
    self.tilt = self
      .tilt
      .clamp(BACKGROUND_WAVE_TILT_MIN, BACKGROUND_WAVE_TILT_MAX);
    self.zoom = self
      .zoom
      .clamp(BACKGROUND_WAVE_ZOOM_MIN, BACKGROUND_WAVE_ZOOM_MAX);
    self.horizon_height = self.horizon_height.clamp(
      BACKGROUND_WAVE_HORIZON_HEIGHT_MIN,
      BACKGROUND_WAVE_HORIZON_HEIGHT_MAX,
    );
    self.fog_depth = self
      .fog_depth
      .clamp(BACKGROUND_WAVE_FOG_DEPTH_MIN, BACKGROUND_WAVE_FOG_DEPTH_MAX);
    self.brightness = self.brightness.clamp(
      BACKGROUND_WAVE_BRIGHTNESS_MIN,
      BACKGROUND_WAVE_BRIGHTNESS_MAX,
    );
    self.opacity = self
      .opacity
      .clamp(BACKGROUND_WAVE_OPACITY_MIN, BACKGROUND_WAVE_OPACITY_MAX);
    self
  }
}

#[cfg(test)]
mod tests;
