//! Unit tests for the Theater state helpers that do not depend on the
//! Leptos reactive runtime being provided (pure data helpers + role
//! permission checks). Signal-touching behaviour lives behind
//! wasm-bindgen tests because they require a Leptos `Owner`.

use super::*;

#[test]
fn role_permission_matrix_matches_req_15_3() {
  assert!(TheaterRole::Owner.can_control_playback());
  assert!(TheaterRole::Owner.can_moderate());

  assert!(!TheaterRole::Admin.can_control_playback());
  assert!(TheaterRole::Admin.can_moderate());

  assert!(!TheaterRole::Viewer.can_control_playback());
  assert!(!TheaterRole::Viewer.can_moderate());
}

#[test]
fn quality_tier_labels_are_human_readable() {
  assert_eq!(QualityTier::HighDefinition.label(), "1080p/30fps");
  assert_eq!(QualityTier::StandardDefinition.label(), "720p/30fps");
  assert_eq!(QualityTier::Low.label(), "480p/15fps");
}

#[test]
fn default_overlay_settings_match_requirements() {
  let settings = TheaterOverlaySettings::default();
  assert!(settings.danmaku_visible);
  assert_eq!(settings.danmaku_opacity, 100);
  assert_eq!(settings.danmaku_font_size, "medium");
  assert_eq!(settings.danmaku_speed, "medium");
  assert_eq!(settings.subtitle.font_size, "medium");
  assert_eq!(settings.subtitle.text_color, "#FFFFFF");
  assert_eq!(settings.subtitle.background_opacity, 40);
  assert_eq!(settings.subtitle.position, SubtitlePosition::Bottom);
}

#[test]
fn subtitle_appearance_serializes_round_trip() {
  let original = SubtitleAppearance {
    font_size: "large".into(),
    text_color: "#FFFF00".into(),
    background_opacity: 60,
    position: SubtitlePosition::Top,
  };
  let json = serde_json::to_string(&original).expect("serialise");
  let round: SubtitleAppearance = serde_json::from_str(&json).expect("deserialise");
  assert_eq!(round, original);
}

#[test]
fn playback_snapshot_equality() {
  let a = PlaybackSnapshot {
    current_time_ms: 1_000,
    duration_ms: 60_000,
    is_paused: false,
  };
  let b = a;
  assert_eq!(a, b);
}
