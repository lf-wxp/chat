//! Settings data types and enumerations.
//!
//! Contains the core value types used by the settings system:
//! [`FontScale`], [`VideoQualityPref`], [`DndWindow`], and
//! [`UserSettings`]. These are pure data — no browser I/O or
//! reactive state lives here.

use serde::{Deserialize, Serialize};

use super::background_waves::WaveConfig;

/// Maximum allowed speaker volume (0.0 – 1.0 inclusive).
pub const VOLUME_MAX: f32 = 1.0;

/// Preferred font size scale. Mapped to the `--font-scale` CSS custom
/// property so every `rem`-based token downsizes / upsizes together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FontScale {
  /// Smaller-than-default (0.9x) — tighter layout for power users.
  Small,
  /// Default scale (1.0x).
  #[default]
  Medium,
  /// Larger-than-default (1.15x) — accessibility-friendly.
  Large,
}

impl FontScale {
  /// Parse from the `<data-font-scale>` attribute / localStorage value.
  #[must_use]
  pub fn parse(value: &str) -> Self {
    match value {
      "small" => Self::Small,
      "large" => Self::Large,
      _ => Self::Medium,
    }
  }

  /// Stable string token used in localStorage and CSS attribute.
  #[must_use]
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Small => "small",
      Self::Medium => "medium",
      Self::Large => "large",
    }
  }

  /// Multiplier applied to the root font size. Maps directly to the
  /// specification's absolute 14 px / 16 px / 18 px targets assuming
  /// a 16 px root baseline (Req 13.2.4).
  #[must_use]
  pub fn scale(self) -> f32 {
    match self {
      Self::Small => 0.875,
      Self::Medium => 1.0,
      Self::Large => 1.125,
    }
  }
}

/// Preferred video capture quality profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VideoQualityPref {
  /// System auto-selects based on network conditions (default).
  /// Falls back to 720p in practice.
  #[default]
  Auto,
  /// ~360p — lowest bandwidth. Saves data on weak networks.
  Low,
  /// ~720p. Matches the baseline `VideoProfile::HIGH`.
  Standard,
  /// ~1080p — requires solid bandwidth and a capable camera.
  High,
}

impl VideoQualityPref {
  /// Parse from the settings form / localStorage value.
  #[must_use]
  pub fn parse(value: &str) -> Self {
    match value {
      "auto" => Self::Auto,
      "low" => Self::Low,
      "high" => Self::High,
      "standard" => Self::Standard,
      _ => Self::Auto,
    }
  }

  /// Stable string token.
  #[must_use]
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Auto => "auto",
      Self::Low => "low",
      Self::Standard => "standard",
      Self::High => "high",
    }
  }
}

/// Wall-clock minute offset from midnight.
///
/// Encodes the do-not-disturb window. The window wraps past midnight
/// when `start > end` (e.g. 22:00 – 07:00).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DndWindow {
  /// Start offset (minutes since 00:00). Defaults to `0`.
  pub start_minutes: u32,
  /// End offset (minutes since 00:00). Defaults to `0`.
  pub end_minutes: u32,
  /// Whether the window is active. When `false`, `start`/`end` are
  /// ignored — this lets the user toggle DND without losing the
  /// previously configured hours.
  pub enabled: bool,
}

impl DndWindow {
  /// Return `true` when `now_minutes` falls inside the configured
  /// window. Handles the wrap-around case (start after end) by
  /// treating the window as two half-open intervals.
  #[must_use]
  pub fn contains(&self, now_minutes: u32) -> bool {
    if !self.enabled {
      return false;
    }
    if self.start_minutes == self.end_minutes {
      return false;
    }
    if self.start_minutes < self.end_minutes {
      now_minutes >= self.start_minutes && now_minutes < self.end_minutes
    } else {
      // Wrap-around window (e.g. 22:00 – 07:00).
      now_minutes >= self.start_minutes || now_minutes < self.end_minutes
    }
  }

  /// Return `true` when the local wall-clock time falls inside the
  /// configured window. Returns `false` when running outside a
  /// browser context (e.g. native unit tests) or when DND is off.
  #[must_use]
  pub fn is_active_now(&self) -> bool {
    if !self.enabled {
      return false;
    }
    if web_sys::window().is_none() {
      return false;
    }
    let date = js_sys::Date::new_0();
    // `getHours` / `getMinutes` return local-time components.
    let hours = date.get_hours();
    let minutes = date.get_minutes();
    let total = hours.saturating_mul(60).saturating_add(minutes);
    self.contains(total)
  }
}

// ── Background settings (plan §7.1 / batch 5) ──────────────────────────

/// Maximum allowed background blur radius (in CSS pixels). Values
/// above this threshold trade too much GPU for too little visual
/// gain and are clamped down by [`BackgroundSettings::sanitised`].
pub const BACKGROUND_BLUR_MAX_PX: u8 = 40;

/// Maximum allowed overlay opacity applied on top of the background
/// image / gradient. Keeps text contrast within WCAG AA even when
/// the user picks a busy image.
pub const BACKGROUND_OVERLAY_ALPHA_MAX: f32 = 0.8;

/// High-level background mode picker. Mirrors the radio group in
/// `BackgroundSection` (batch 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundMode {
  /// Use one of the built-in themed presets. Requires
  /// [`BackgroundSettings::preset_id`] to be set; when missing the
  /// sanitiser reverts to [`Self::Preset`] with the default preset.
  #[default]
  Preset,
  /// Paint a single flat colour sourced from
  /// [`BackgroundSettings::solid_color`].
  Solid,
  /// Paint a multi-stop gradient sourced from
  /// [`BackgroundSettings::gradient`].
  Gradient,
  /// Render a user-supplied image persisted in IndexedDB under
  /// [`BackgroundSettings::image_blob_key`] (batch 6 wires the
  /// storage layer; batch 5 only carries the key).
  Image,
}

impl BackgroundMode {
  /// Parse from the form / localStorage value. Unknown tokens fall
  /// back to [`Self::Preset`] so the UI always has a safe default.
  #[must_use]
  pub fn parse(value: &str) -> Self {
    match value {
      "solid" => Self::Solid,
      "gradient" => Self::Gradient,
      "image" => Self::Image,
      _ => Self::Preset,
    }
  }

  /// Stable token used in localStorage and form inputs.
  #[must_use]
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Preset => "preset",
      Self::Solid => "solid",
      Self::Gradient => "gradient",
      Self::Image => "image",
    }
  }
}

/// Which WebGL background effects to render. Independent of
/// [`BackgroundMode`] — the effects layer sits on top of whatever
/// static background (preset / solid / gradient / image) is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundEffects {
  /// All three effects layered (waves + rays + particles).
  #[default]
  All,
  /// Only the raymarched gradient waves.
  Waves,
  /// Only the volumetric light rays.
  Rays,
  /// Only the drifting particles.
  Particles,
  /// No WebGL effects — plain static background.
  None,
}

impl BackgroundEffects {
  #[must_use]
  pub fn parse(value: &str) -> Self {
    match value {
      "waves" => Self::Waves,
      "rays" => Self::Rays,
      "particles" => Self::Particles,
      "none" => Self::None,
      _ => Self::All,
    }
  }

  #[must_use]
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::All => "all",
      Self::Waves => "waves",
      Self::Rays => "rays",
      Self::Particles => "particles",
      Self::None => "none",
    }
  }
}

/// Gradient shape. Kept narrow on purpose — radial + linear cover
/// 95 % of decorative backgrounds while keeping the CSS generator
/// trivial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GradientKind {
  /// Linear gradient with user-selectable angle.
  #[default]
  Linear,
  /// Radial gradient anchored at the viewport centre.
  Radial,
}

/// Single colour stop along a gradient. `offset` is stored
/// normalised (`0.0..=1.0`) so the renderer can translate it into
/// CSS `%` without losing precision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GradientStop {
  /// CSS colour token. Validated only in shape (non-empty); the
  /// actual parsing is delegated to the browser.
  pub color: String,
  /// Offset along the gradient axis, `0.0..=1.0`.
  pub offset: f32,
}

impl Default for GradientStop {
  fn default() -> Self {
    Self {
      color: "#3b82f6".into(),
      offset: 0.0,
    }
  }
}

/// Multi-stop gradient specification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GradientSpec {
  /// Gradient shape.
  pub kind: GradientKind,
  /// Angle in degrees for [`GradientKind::Linear`]. Ignored by
  /// [`GradientKind::Radial`]. Clamped to `0..=360` by
  /// [`GradientSpec::sanitised`].
  pub angle_deg: u16,
  /// At least two stops must be present — the sanitiser fills in
  /// defaults when the user state is corrupted.
  pub stops: Vec<GradientStop>,
}

impl Default for GradientSpec {
  fn default() -> Self {
    Self {
      kind: GradientKind::default(),
      angle_deg: 180,
      stops: vec![
        GradientStop {
          color: "#3b82f6".into(),
          offset: 0.0,
        },
        GradientStop {
          color: "#ec4899".into(),
          offset: 1.0,
        },
      ],
    }
  }
}

impl GradientSpec {
  /// Clamp `angle_deg` into `0..=360` and normalise every stop.
  /// Guarantees at least two stops so the CSS generator never emits
  /// an invalid `linear-gradient(...)` value.
  #[must_use]
  pub fn sanitised(mut self) -> Self {
    if self.angle_deg > 360 {
      self.angle_deg = 360;
    }
    for stop in &mut self.stops {
      stop.offset = stop.offset.clamp(0.0, 1.0);
      if stop.color.trim().is_empty() {
        stop.color = "#3b82f6".into();
      }
    }
    while self.stops.len() < 2 {
      self.stops.push(GradientStop::default());
    }
    self
  }

  /// Render this spec as a CSS `background-image` value.
  #[must_use]
  pub fn to_css(&self) -> String {
    let stops = self
      .stops
      .iter()
      .map(|s| format!("{} {:.1}%", s.color, s.offset * 100.0))
      .collect::<Vec<_>>()
      .join(", ");
    match self.kind {
      GradientKind::Linear => format!("linear-gradient({}deg, {})", self.angle_deg, stops),
      GradientKind::Radial => format!("radial-gradient(circle at center, {stops})"),
    }
  }
}

/// Light/dark variant payload. Only carries the visual knobs — the
/// shared overlay / blur / theme-aware flag stay on the parent
/// [`BackgroundSettings`] so toggling theme-aware mode does not
/// force users to re-configure those cross-cutting sliders.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BackgroundVariantData {
  /// Mode of the variant.
  pub mode: BackgroundMode,
  /// Preset identifier when `mode == Preset`.
  pub preset_id: Option<String>,
  /// Solid colour token when `mode == Solid`.
  pub solid_color: Option<String>,
  /// Gradient payload when `mode == Gradient`.
  pub gradient: Option<GradientSpec>,
  /// IndexedDB blob key when `mode == Image`.
  pub image_blob_key: Option<String>,
}

impl BackgroundVariantData {
  /// Clamp the variant to a valid shape, mirroring the top-level
  /// sanitiser rules for mode ↔ payload coherence.
  #[must_use]
  pub fn sanitised(mut self) -> Self {
    if let Some(g) = self.gradient {
      self.gradient = Some(g.sanitised());
    }
    match self.mode {
      BackgroundMode::Solid if self.solid_color.is_none() => {
        self.mode = BackgroundMode::Preset;
      }
      BackgroundMode::Gradient if self.gradient.is_none() => {
        self.mode = BackgroundMode::Preset;
      }
      // Image mode is kept even without a blob key so the upload UI
      // remains visible while the user picks a file.
      _ => {}
    }
    self
  }
}

/// Root background configuration. Persisted inside [`UserSettings`]
/// so the existing localStorage + serde migration path carries it
/// "for free". IndexedDB (batch 6) holds the actual image blobs;
/// this struct only stores the key pointing into that store.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackgroundSettings {
  /// Active visual mode.
  pub mode: BackgroundMode,
  /// Preset identifier (e.g. `"aurora"`, `"mica-blue"`) used when
  /// `mode == Preset`.
  pub preset_id: Option<String>,
  /// Solid fill colour used when `mode == Solid`.
  pub solid_color: Option<String>,
  /// Gradient payload used when `mode == Gradient`.
  pub gradient: Option<GradientSpec>,
  /// IndexedDB blob key used when `mode == Image`. Shape:
  /// `user_bg_light` / `user_bg_dark`; the renderer resolves it
  /// into an object URL at display time.
  pub image_blob_key: Option<String>,
  /// Backdrop blur radius in CSS pixels, clamped to
  /// `0..=BACKGROUND_BLUR_MAX_PX`.
  pub blur_px: u8,
  /// Overlay opacity applied above the background image / gradient,
  /// clamped to `0.0..=BACKGROUND_OVERLAY_ALPHA_MAX`.
  pub overlay_alpha: f32,
  /// When `true`, the dark-theme variant is held in `dark` and the
  /// top-level fields describe the light theme only. When `false`,
  /// the top-level fields apply to both themes.
  pub theme_aware: bool,
  /// Dark-theme-specific payload. `None` unless `theme_aware` has
  /// been switched on at least once. Boxed to keep
  /// `BackgroundSettings` small (the enum-free branch dominates).
  pub dark: Option<Box<BackgroundVariantData>>,
  /// Which WebGL effects to render on top of the static background.
  /// Defaults to [`BackgroundEffects::All`].
  #[serde(default)]
  pub effects: BackgroundEffects,
  /// Gradient Waves shader configuration (scale, ratio, speed,
  /// swell, turbulence, tilt, zoom, horizon height, fog depth,
  /// brightness, opacity). See [`WaveConfig`]. `#[serde(default)]`
  /// keeps older persisted payloads (predating this field)
  /// deserialising with the original hard-coded shader look.
  #[serde(default)]
  pub waves: WaveConfig,
}

impl Default for BackgroundSettings {
  fn default() -> Self {
    // Preset mode with no explicit id means "use the tokens.css
    // default gradient" — the renderer leaves `--app-bg-*` untouched.
    Self {
      mode: BackgroundMode::Preset,
      preset_id: None,
      solid_color: None,
      gradient: None,
      image_blob_key: None,
      blur_px: 0,
      overlay_alpha: 0.2,
      theme_aware: false,
      dark: None,
      effects: BackgroundEffects::All,
      waves: WaveConfig::default(),
    }
  }
}

impl BackgroundSettings {
  /// Normalise the settings to a coherent state.
  ///
  /// * Clamp `blur_px` and `overlay_alpha` into their valid ranges.
  /// * Sanitise the embedded gradient spec when present.
  /// * When the mode's required payload is missing (Solid without a
  ///   colour, Gradient without stops), fall back to
  ///   [`BackgroundMode::Preset`]. Image mode is intentionally
  ///   exempt: the user must remain in Image mode so the upload UI
  ///   stays visible before a file is selected.
  /// * When `theme_aware` is off, drop any stale dark variant so
  ///   serde output stays minimal.
  #[must_use]
  pub fn sanitised(mut self) -> Self {
    if self.blur_px > BACKGROUND_BLUR_MAX_PX {
      self.blur_px = BACKGROUND_BLUR_MAX_PX;
    }
    self.overlay_alpha = self.overlay_alpha.clamp(0.0, BACKGROUND_OVERLAY_ALPHA_MAX);
    self.waves = self.waves.sanitised();

    if let Some(g) = self.gradient {
      self.gradient = Some(g.sanitised());
    }

    match self.mode {
      BackgroundMode::Solid if self.solid_color.is_none() => {
        self.mode = BackgroundMode::Preset;
      }
      BackgroundMode::Gradient if self.gradient.is_none() => {
        self.mode = BackgroundMode::Preset;
      }
      // Image mode is kept even without a blob key so the upload UI
      // remains visible while the user picks a file.
      _ => {}
    }

    if self.theme_aware {
      if let Some(dark) = self.dark {
        self.dark = Some(Box::new(dark.sanitised()));
      }
    } else {
      self.dark = None;
    }

    self
  }

  /// Resolve which variant payload applies to the currently active
  /// theme. Returns a lightweight view that borrows from `self` so
  /// callers never have to clone the potentially large image key or
  /// gradient stops.
  #[must_use]
  pub fn active_variant(&self, is_dark_theme: bool) -> BackgroundVariantView<'_> {
    if self.theme_aware
      && is_dark_theme
      && let Some(dark) = self.dark.as_deref()
    {
      return BackgroundVariantView {
        mode: dark.mode,
        preset_id: dark.preset_id.as_deref(),
        solid_color: dark.solid_color.as_deref(),
        gradient: dark.gradient.as_ref(),
        image_blob_key: dark.image_blob_key.as_deref(),
      };
    }
    BackgroundVariantView {
      mode: self.mode,
      preset_id: self.preset_id.as_deref(),
      solid_color: self.solid_color.as_deref(),
      gradient: self.gradient.as_ref(),
      image_blob_key: self.image_blob_key.as_deref(),
    }
  }

  /// Render the active variant as a list of CSS custom properties
  /// (`--app-bg-*`). The list is intentionally small so batch 7's
  /// `AppBg` component can iterate and `setProperty` in a single
  /// pass. Returning tuples avoids a public dependency on any DOM
  /// API and keeps this function unit-testable on native.
  #[must_use]
  pub fn to_css_vars(&self, is_dark_theme: bool) -> Vec<(&'static str, String)> {
    let variant = self.active_variant(is_dark_theme);
    let mut vars: Vec<(&'static str, String)> = Vec::with_capacity(4);

    match variant.mode {
      BackgroundMode::Preset => {
        // Preset mode defers to tokens.css defaults. We still emit
        // the mode hint so CSS can key off `data-app-bg-mode`.
      }
      BackgroundMode::Solid => {
        if let Some(color) = variant.solid_color {
          vars.push(("--app-bg-solid", color.to_owned()));
        }
      }
      BackgroundMode::Gradient => {
        if let Some(g) = variant.gradient {
          vars.push(("--app-bg-gradient", g.to_css()));
        }
      }
      BackgroundMode::Image => {
        // Image mode fills `--app-bg-image` at object-URL time; the
        // blob → URL round-trip is owned by batch 6. Here we only
        // advertise that image mode is active via the mode token.
      }
    }

    vars.push(("--app-bg-blur", format!("{}px", self.blur_px)));
    vars.push((
      "--app-bg-overlay",
      format!("rgb(0 0 0 / {:.3})", self.overlay_alpha),
    ));

    vars
  }
}

/// Zero-copy view over the variant that applies for the active
/// theme. Returned by [`BackgroundSettings::active_variant`].
#[derive(Debug)]
pub struct BackgroundVariantView<'a> {
  pub mode: BackgroundMode,
  pub preset_id: Option<&'a str>,
  pub solid_color: Option<&'a str>,
  pub gradient: Option<&'a GradientSpec>,
  pub image_blob_key: Option<&'a str>,
}

/// Full user-settings record. Serialised to JSON in `localStorage`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserSettings {
  /// Preferred video input device id (MediaDeviceInfo.deviceId).
  pub default_camera: Option<String>,
  /// Preferred audio input device id.
  pub default_microphone: Option<String>,
  /// Preferred audio output device id (falls back to default output).
  pub default_speaker: Option<String>,
  /// Speaker volume scalar (0.0 – 1.0).
  pub speaker_volume: f32,
  /// Microphone volume scalar (0.0 – 1.0) (Req 13.1.3).
  pub microphone_volume: f32,
  /// Preferred video capture quality.
  pub video_quality: VideoQualityPref,
  /// Typography scale.
  pub font_scale: FontScale,
  /// Whether the user broadcasts "online" status to peers.
  pub online_status_visible: bool,
  /// Whether read receipts are sent.
  pub read_receipts: bool,
  /// Whether incoming chat messages trigger a desktop notification.
  pub message_notifications: bool,
  /// Whether incoming call invites trigger a desktop notification.
  pub call_notifications: bool,
  /// Quiet-hours configuration.
  pub dnd: DndWindow,
  /// Message retention window — type-safe enum replaces the former
  /// free-form `String` field (P2-11). Reuses the canonical
  /// [`crate::persistence::RetentionPolicy`] type.
  pub retention: crate::persistence::RetentionPolicy,
  /// Whether the Mica-style glass effect is enabled. Defaults to
  /// `true`; mirrored to `<html data-glass="on|off">` by the root
  /// App effect so effects.css picks up the override without any
  /// additional plumbing. Uses `#[serde(default = ...)]` so existing
  /// persisted settings predating this field keep deserialising.
  #[serde(default = "default_true")]
  pub glass_enabled: bool,
  /// Whether decorative "cool" animations (pulse, drift, shimmer,
  /// stream-line) are enabled. Defaults to `true`. Mirrors to
  /// `<html data-motion="on|off">`.
  #[serde(default = "default_true")]
  pub motion_enabled: bool,
  /// Background configuration (plan §7 / batch 5). Defaults to the
  /// built-in preset. `#[serde(default)]` keeps older persisted
  /// settings deserialising without this field present.
  #[serde(default)]
  pub background: BackgroundSettings,
}

impl Default for UserSettings {
  fn default() -> Self {
    Self {
      default_camera: None,
      default_microphone: None,
      default_speaker: None,
      speaker_volume: 1.0,
      microphone_volume: 1.0,
      video_quality: VideoQualityPref::default(),
      font_scale: FontScale::default(),
      online_status_visible: true,
      read_receipts: true,
      message_notifications: true,
      call_notifications: true,
      dnd: DndWindow::default(),
      retention: crate::persistence::RetentionPolicy::default(),
      glass_enabled: true,
      motion_enabled: true,
      background: BackgroundSettings::default(),
    }
  }
}

/// Serde default helper for `bool` fields that should default to
/// `true`. Avoids repeating the same closure-like function literal
/// at every call site and keeps the `#[serde(default = ...)]`
/// attribute expressive.
fn default_true() -> bool {
  true
}

impl UserSettings {
  /// Clamp all numeric fields to their valid ranges. Called after
  /// deserialisation so a hand-edited localStorage value cannot push
  /// the runtime into an inconsistent state.
  #[must_use]
  pub fn sanitised(mut self) -> Self {
    self.speaker_volume = self.speaker_volume.clamp(0.0, VOLUME_MAX);
    self.microphone_volume = self.microphone_volume.clamp(0.0, VOLUME_MAX);
    self.dnd.start_minutes = self.dnd.start_minutes.min(24 * 60 - 1);
    self.dnd.end_minutes = self.dnd.end_minutes.min(24 * 60 - 1);
    self.background = self.background.sanitised();
    self
  }
}
