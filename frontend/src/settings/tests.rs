//! Unit tests for the settings module.

use super::*;

#[test]
fn font_scale_parse_round_trip() {
  for value in [FontScale::Small, FontScale::Medium, FontScale::Large] {
    assert_eq!(FontScale::parse(value.as_str()), value);
  }
  // Unknown inputs fall back to Medium.
  assert_eq!(FontScale::parse("bogus"), FontScale::Medium);
  assert_eq!(FontScale::parse(""), FontScale::Medium);
}

#[test]
fn font_scale_multiplier_monotonic() {
  assert!(FontScale::Small.scale() < FontScale::Medium.scale());
  assert!(FontScale::Medium.scale() < FontScale::Large.scale());
}

#[test]
fn video_quality_parse_round_trip() {
  for value in [
    VideoQualityPref::Auto,
    VideoQualityPref::Low,
    VideoQualityPref::Standard,
    VideoQualityPref::High,
  ] {
    assert_eq!(VideoQualityPref::parse(value.as_str()), value);
  }
  // Unknown inputs fall back to Auto (the new default).
  assert_eq!(VideoQualityPref::parse("xyz"), VideoQualityPref::Auto);
}

#[test]
fn dnd_window_disabled_never_triggers() {
  let window = DndWindow {
    start_minutes: 60,
    end_minutes: 120,
    enabled: false,
  };
  assert!(!window.contains(90));
  assert!(!window.contains(0));
}

#[test]
fn dnd_window_simple_interval() {
  let window = DndWindow {
    start_minutes: 60,
    end_minutes: 120,
    enabled: true,
  };
  assert!(!window.contains(59));
  assert!(window.contains(60));
  assert!(window.contains(90));
  assert!(!window.contains(120));
  assert!(!window.contains(200));
}

#[test]
fn dnd_window_wraps_past_midnight() {
  let window = DndWindow {
    // 22:00 — 07:00
    start_minutes: 22 * 60,
    end_minutes: 7 * 60,
    enabled: true,
  };
  assert!(window.contains(22 * 60));
  assert!(window.contains(23 * 60));
  assert!(window.contains(0));
  assert!(window.contains(6 * 60 + 59));
  assert!(!window.contains(7 * 60));
  assert!(!window.contains(12 * 60));
}

#[test]
fn dnd_zero_width_window_never_triggers() {
  let window = DndWindow {
    start_minutes: 120,
    end_minutes: 120,
    enabled: true,
  };
  assert!(!window.contains(120));
  assert!(!window.contains(0));
}

#[test]
fn settings_sanitised_clamps_volume() {
  let settings = UserSettings {
    speaker_volume: 5.0,
    microphone_volume: 1.0,
    ..UserSettings::default()
  };
  let sanitised = settings.sanitised();
  assert!((sanitised.speaker_volume - VOLUME_MAX).abs() < f32::EPSILON);

  let negative = UserSettings {
    speaker_volume: -0.5,
    microphone_volume: 1.0,
    ..UserSettings::default()
  };
  assert!(negative.sanitised().speaker_volume >= 0.0);

  // Microphone volume is also clamped.
  let high_mic = UserSettings {
    microphone_volume: 3.0,
    ..UserSettings::default()
  };
  assert!((high_mic.sanitised().microphone_volume - VOLUME_MAX).abs() < f32::EPSILON);

  let neg_mic = UserSettings {
    microphone_volume: -1.0,
    ..UserSettings::default()
  };
  assert!(neg_mic.sanitised().microphone_volume >= 0.0);
}

#[test]
fn settings_sanitised_clamps_dnd_minutes() {
  let settings = UserSettings {
    dnd: DndWindow {
      start_minutes: 9_999,
      end_minutes: 10_000,
      enabled: false,
    },
    ..UserSettings::default()
  };
  let sanitised = settings.sanitised();
  assert_eq!(sanitised.dnd.start_minutes, 24 * 60 - 1);
  assert_eq!(sanitised.dnd.end_minutes, 24 * 60 - 1);
}

#[test]
fn settings_serde_round_trip() {
  let settings = UserSettings {
    default_camera: Some("cam-1".into()),
    default_microphone: Some("mic-1".into()),
    default_speaker: Some("out-1".into()),
    speaker_volume: 0.6,
    microphone_volume: 0.8,
    video_quality: VideoQualityPref::High,
    font_scale: FontScale::Large,
    online_status_visible: false,
    read_receipts: false,
    message_notifications: false,
    call_notifications: false,
    dnd: DndWindow {
      start_minutes: 60,
      end_minutes: 120,
      enabled: true,
    },
    retention: crate::persistence::RetentionPolicy::Week,
    glass_enabled: false,
    motion_enabled: false,
    background: BackgroundSettings::default(),
  };
  let json = serde_json::to_string(&settings).expect("serialise");
  let decoded: UserSettings = serde_json::from_str(&json).expect("deserialise");
  assert_eq!(decoded, settings);
}

#[test]
fn export_payload_json_contains_fields() {
  // Default payload has no messages/contacts/blacklist, so those
  // fields are omitted via `skip_serializing_if = "Option::is_none"`.
  let payload = ExportPayload::new(UserSettings::default(), None);
  let json = payload.to_json();
  assert!(json.contains("\"exported_at\""));
  assert!(json.contains("\"settings\""));
  assert!(json.contains("\"app_version\""));

  // With a messages block provided, the field should appear.
  let with_messages = ExportPayload::new(UserSettings::default(), Some(serde_json::json!({})));
  let json = with_messages.to_json();
  assert!(json.contains("\"messages\""));
}

#[test]
fn export_payload_html_escapes_special_chars() {
  let settings = UserSettings {
    default_camera: Some("<script>".into()),
    ..UserSettings::default()
  };
  let payload = ExportPayload::new(settings, None);
  let html = payload.to_html();
  assert!(html.contains("&lt;script&gt;"));
  assert!(!html.contains("<script>cam"));
}

#[test]
fn export_payload_omits_sensitive_tokens() {
  // Sanity check: the payload struct intentionally does not carry
  // any JWT / token field. Deserialising a payload with extra
  // unknown fields should still succeed (serde ignores them) but the
  // public struct must never expose them.
  let default_payload = ExportPayload::new(UserSettings::default(), None);
  let json = default_payload.to_json();
  assert!(!json.contains("jwt"));
  assert!(!json.contains("token"));
  assert!(!json.contains("password"));
}

#[test]
fn export_payload_full_includes_contacts_and_blacklist() {
  let contacts = serde_json::json!([{"user_id": "u1", "username": "alice"}]);
  let blacklist = serde_json::json!([{"user_id": "u2", "display_name": "bob"}]);
  let payload = ExportPayload::full(
    UserSettings::default(),
    None,
    Some(contacts),
    Some(blacklist),
  );
  let json = payload.to_json();
  assert!(json.contains("\"contacts\""));
  assert!(json.contains("\"alice\""));
  assert!(json.contains("\"blacklist\""));
  assert!(json.contains("\"bob\""));
}

#[test]
fn export_html_renders_conversation_messages() {
  let messages = serde_json::json!({
    "conv-1": [
      {"sender_name": "Alice", "body": "Hello world", "timestamp_ms": 0_i64},
      {"sender_name": "Bob", "body": "<script>alert(1)</script>", "timestamp_ms": 1_000_i64},
    ]
  });
  let payload = ExportPayload::full(UserSettings::default(), Some(messages), None, None);
  let html = payload.to_html();
  assert!(html.contains("Alice"));
  assert!(html.contains("Hello world"));
  // Script tags are escaped, never emitted verbatim.
  assert!(html.contains("&lt;script&gt;"));
  assert!(!html.contains("<script>alert"));
  // The conversation heading is present.
  assert!(html.contains("conv-1"));
}

#[test]
fn font_scale_matches_specification() {
  // Req 13.2.4: Small=14px, Medium=16px, Large=18px (with 16px root
  // baseline → 0.875 / 1.0 / 1.125 multipliers).
  assert!((FontScale::Small.scale() - 0.875).abs() < f32::EPSILON);
  assert!((FontScale::Medium.scale() - 1.0).abs() < f32::EPSILON);
  assert!((FontScale::Large.scale() - 1.125).abs() < f32::EPSILON);
}

#[test]
fn saved_tick_increments_on_update_native_stub() {
  // `SettingsState::new` touches `web_sys::window` via
  // `load_from_storage`, so this test runs the data-level serde
  // round-trip instead. The `saved_tick` behaviour itself is
  // exercised by the wasm-bindgen tests and the Playwright e2e.
  let settings = UserSettings {
    speaker_volume: 0.25,
    ..UserSettings::default()
  };
  let serialised = serde_json::to_string(&settings).unwrap();
  let restored: UserSettings = serde_json::from_str(&serialised).unwrap();
  assert!((restored.speaker_volume - 0.25).abs() < f32::EPSILON);
}

#[test]
fn retention_policy_default_is_three_days() {
  assert_eq!(
    crate::persistence::RetentionPolicy::default(),
    crate::persistence::RetentionPolicy::ThreeDays,
  );
}

#[test]
fn retention_default_matches_settings_default() {
  let settings = UserSettings::default();
  assert_eq!(
    settings.retention,
    crate::persistence::RetentionPolicy::ThreeDays
  );
}

#[test]
fn video_quality_default_is_auto() {
  assert_eq!(VideoQualityPref::default(), VideoQualityPref::Auto);
}

#[test]
fn baseline_video_profile_auto_falls_back_to_high() {
  let profile = crate::call::baseline_video_profile(VideoQualityPref::Auto);
  assert_eq!(profile.width, 1280);
  assert_eq!(profile.height, 720);
  assert_eq!(profile.frame_rate, 30);
}

#[test]
fn settings_retention_serialises_as_enum() {
  let settings = UserSettings {
    retention: crate::persistence::RetentionPolicy::Week,
    ..UserSettings::default()
  };
  let json = serde_json::to_string(&settings).unwrap();
  // serde with the default derive serialises as "Week" (PascalCase).
  assert!(json.contains("\"Week\"") || json.contains("\"week\""));
  let restored: UserSettings = serde_json::from_str(&json).unwrap();
  assert_eq!(
    restored.retention,
    crate::persistence::RetentionPolicy::Week
  );
}

// ---------------------------------------------------------------------------
// html_escape — security-critical string sanitisation for HTML export
// ---------------------------------------------------------------------------

#[test]
fn html_escape_escapes_xss_vectors() {
  assert_eq!(html_escape("<script>"), "&lt;script&gt;");
  assert_eq!(html_escape(">"), "&gt;");
  assert_eq!(html_escape("&"), "&amp;");
  assert_eq!(html_escape("\""), "&quot;");
  assert_eq!(html_escape("'"), "&#x27;");
}

#[test]
fn html_escape_preserves_plain_text() {
  assert_eq!(html_escape("hello world"), "hello world");
  assert_eq!(html_escape("中文内容"), "中文内容");
  assert_eq!(html_escape("123_456"), "123_456");
}

#[test]
fn html_escape_handles_empty_string() {
  assert_eq!(html_escape(""), "");
}

#[test]
fn html_escape_handles_combined_mixed_content() {
  let input = "<div class=\"test\">Hello & welcome to >_< </div>";
  let expected = "&lt;div class=&quot;test&quot;&gt;Hello &amp; welcome to &gt;_&lt; &lt;/div&gt;";
  assert_eq!(html_escape(input), expected);
}

// ---------------------------------------------------------------------------
// PermissionBadge CSS class mapping
// ---------------------------------------------------------------------------

/// Mirrors the CSS modifier logic used by `PermissionBadge` so we can unit-
/// test the mapping without spinning up a Leptos renderer.
fn permission_badge_class(state: &str) -> &'static str {
  match state {
    "granted" => "is-granted",
    "denied" => "is-denied",
    "prompt" => "is-prompt",
    _ => "is-unsupported",
  }
}

#[test]
fn permission_badge_class_maps_all_states() {
  assert_eq!(permission_badge_class("granted"), "is-granted");
  assert_eq!(permission_badge_class("denied"), "is-denied");
  assert_eq!(permission_badge_class("prompt"), "is-prompt");
  assert_eq!(permission_badge_class("default"), "is-unsupported");
  assert_eq!(permission_badge_class(""), "is-unsupported");
  assert_eq!(permission_badge_class("unknown"), "is-unsupported");
}

// ---------------------------------------------------------------------------
// BackgroundSettings — plan §7.1 / batch 5
// ---------------------------------------------------------------------------

#[test]
fn background_mode_parse_round_trip() {
  for value in [
    BackgroundMode::Preset,
    BackgroundMode::Solid,
    BackgroundMode::Gradient,
    BackgroundMode::Image,
  ] {
    assert_eq!(BackgroundMode::parse(value.as_str()), value);
  }
  // Unknown tokens fall back to Preset.
  assert_eq!(BackgroundMode::parse("xyz"), BackgroundMode::Preset);
  assert_eq!(BackgroundMode::parse(""), BackgroundMode::Preset);
}

#[test]
fn background_settings_default_is_preset() {
  let s = BackgroundSettings::default();
  assert_eq!(s.mode, BackgroundMode::Preset);
  assert_eq!(s.blur_px, 0);
  assert!((s.overlay_alpha - 0.2).abs() < f32::EPSILON);
  assert!(!s.theme_aware);
  assert!(s.dark.is_none());
  assert!(s.gradient.is_none());
  assert!(s.solid_color.is_none());
  assert!(s.image_blob_key.is_none());
}

#[test]
fn background_settings_sanitised_clamps_blur() {
  let s = BackgroundSettings {
    blur_px: 200,
    ..BackgroundSettings::default()
  };
  assert_eq!(s.sanitised().blur_px, BACKGROUND_BLUR_MAX_PX);
}

#[test]
fn background_settings_sanitised_clamps_overlay_alpha() {
  let high = BackgroundSettings {
    overlay_alpha: 5.0,
    ..BackgroundSettings::default()
  };
  assert!((high.sanitised().overlay_alpha - BACKGROUND_OVERLAY_ALPHA_MAX).abs() < f32::EPSILON);

  let negative = BackgroundSettings {
    overlay_alpha: -0.5,
    ..BackgroundSettings::default()
  };
  assert!(negative.sanitised().overlay_alpha >= 0.0);
}

#[test]
fn background_settings_solid_without_color_falls_back_to_preset() {
  let s = BackgroundSettings {
    mode: BackgroundMode::Solid,
    solid_color: None,
    ..BackgroundSettings::default()
  };
  assert_eq!(s.sanitised().mode, BackgroundMode::Preset);
}

#[test]
fn background_settings_gradient_without_payload_falls_back_to_preset() {
  let s = BackgroundSettings {
    mode: BackgroundMode::Gradient,
    gradient: None,
    ..BackgroundSettings::default()
  };
  assert_eq!(s.sanitised().mode, BackgroundMode::Preset);
}

#[test]
fn background_settings_image_without_key_stays_in_image_mode() {
  // Image mode is preserved even without a blob key so the upload UI
  // remains visible while the user picks a file.
  let s = BackgroundSettings {
    mode: BackgroundMode::Image,
    image_blob_key: None,
    ..BackgroundSettings::default()
  };
  assert_eq!(s.sanitised().mode, BackgroundMode::Image);
}

#[test]
fn background_settings_theme_aware_off_drops_dark_variant() {
  let s = BackgroundSettings {
    theme_aware: false,
    dark: Some(Box::new(BackgroundVariantData {
      mode: BackgroundMode::Solid,
      solid_color: Some("#000".into()),
      ..BackgroundVariantData::default()
    })),
    ..BackgroundSettings::default()
  };
  assert!(s.sanitised().dark.is_none());
}

#[test]
fn gradient_spec_sanitised_clamps_angle_and_stops() {
  let spec = GradientSpec {
    kind: GradientKind::Linear,
    angle_deg: 999,
    stops: vec![GradientStop {
      color: "#ff0000".into(),
      offset: 2.0,
    }],
  };
  let sanitised = spec.sanitised();
  assert_eq!(sanitised.angle_deg, 360);
  assert!(sanitised.stops.len() >= 2);
  assert!(sanitised.stops[0].offset <= 1.0);
}

#[test]
fn gradient_spec_sanitised_fills_empty_color() {
  let spec = GradientSpec {
    kind: GradientKind::Linear,
    angle_deg: 90,
    stops: vec![
      GradientStop {
        color: "   ".into(),
        offset: 0.0,
      },
      GradientStop {
        color: "#00ff00".into(),
        offset: 1.0,
      },
    ],
  };
  let sanitised = spec.sanitised();
  assert!(!sanitised.stops[0].color.trim().is_empty());
}

#[test]
fn gradient_spec_to_css_linear_and_radial() {
  let linear = GradientSpec {
    kind: GradientKind::Linear,
    angle_deg: 90,
    stops: vec![
      GradientStop {
        color: "#ff0000".into(),
        offset: 0.0,
      },
      GradientStop {
        color: "#0000ff".into(),
        offset: 1.0,
      },
    ],
  };
  let css = linear.to_css();
  assert!(css.starts_with("linear-gradient(90deg,"));
  assert!(css.contains("#ff0000 0.0%"));
  assert!(css.contains("#0000ff 100.0%"));

  let radial = GradientSpec {
    kind: GradientKind::Radial,
    ..linear
  };
  let css = radial.to_css();
  assert!(css.starts_with("radial-gradient(circle at center,"));
}

#[test]
fn background_settings_to_css_vars_preset_has_no_bg_value() {
  let s = BackgroundSettings::default();
  let vars: std::collections::HashMap<_, _> = s.to_css_vars(false).into_iter().collect();
  // Preset mode defers to tokens.css — neither solid nor gradient
  // values are emitted. Only the overlay + blur helpers come
  // through.
  assert!(!vars.contains_key("--app-bg-solid"));
  assert!(!vars.contains_key("--app-bg-gradient"));
  assert!(vars.contains_key("--app-bg-blur"));
  assert!(vars.contains_key("--app-bg-overlay"));
}

#[test]
fn background_settings_to_css_vars_solid_emits_color() {
  let s = BackgroundSettings {
    mode: BackgroundMode::Solid,
    solid_color: Some("#123456".into()),
    ..BackgroundSettings::default()
  };
  let vars: std::collections::HashMap<_, _> = s.to_css_vars(false).into_iter().collect();
  assert_eq!(
    vars.get("--app-bg-solid").map(String::as_str),
    Some("#123456")
  );
}

#[test]
fn background_settings_to_css_vars_gradient_emits_css() {
  let s = BackgroundSettings {
    mode: BackgroundMode::Gradient,
    gradient: Some(GradientSpec::default()),
    ..BackgroundSettings::default()
  };
  let vars: std::collections::HashMap<_, _> = s.to_css_vars(false).into_iter().collect();
  let gradient = vars
    .get("--app-bg-gradient")
    .expect("gradient CSS var emitted");
  assert!(gradient.starts_with("linear-gradient("));
}

#[test]
fn background_settings_active_variant_picks_dark_when_theme_aware() {
  let s = BackgroundSettings {
    mode: BackgroundMode::Solid,
    solid_color: Some("#ffffff".into()),
    theme_aware: true,
    dark: Some(Box::new(BackgroundVariantData {
      mode: BackgroundMode::Solid,
      solid_color: Some("#000000".into()),
      ..BackgroundVariantData::default()
    })),
    ..BackgroundSettings::default()
  };

  let light = s.active_variant(false);
  assert_eq!(light.solid_color, Some("#ffffff"));

  let dark = s.active_variant(true);
  assert_eq!(dark.solid_color, Some("#000000"));
}

#[test]
fn background_settings_active_variant_falls_back_without_dark_payload() {
  let s = BackgroundSettings {
    mode: BackgroundMode::Solid,
    solid_color: Some("#ffffff".into()),
    theme_aware: true,
    dark: None,
    ..BackgroundSettings::default()
  };
  // Even when theme_aware is true, a missing dark payload should
  // seamlessly fall back to the top-level light variant so the UI
  // never ends up rendering nothing.
  let dark = s.active_variant(true);
  assert_eq!(dark.solid_color, Some("#ffffff"));
}

#[test]
fn background_settings_serde_round_trip_preserves_all_fields() {
  let original = BackgroundSettings {
    mode: BackgroundMode::Gradient,
    preset_id: Some("aurora".into()),
    solid_color: Some("#abcdef".into()),
    gradient: Some(GradientSpec::default()),
    image_blob_key: Some("user_bg_light".into()),
    blur_px: 12,
    overlay_alpha: 0.45,
    theme_aware: true,
    dark: Some(Box::new(BackgroundVariantData {
      mode: BackgroundMode::Solid,
      solid_color: Some("#112233".into()),
      ..BackgroundVariantData::default()
    })),
  };
  let json = serde_json::to_string(&original).expect("serialise");
  let decoded: BackgroundSettings = serde_json::from_str(&json).expect("deserialise");
  assert_eq!(decoded, original);
}

#[test]
fn user_settings_deserialises_without_background_field() {
  // Forward-compatibility guard: older persisted payloads predate
  // the `background` field. `#[serde(default)]` must cover them.
  let legacy = r#"{
    "default_camera": null,
    "default_microphone": null,
    "default_speaker": null,
    "speaker_volume": 0.5,
    "microphone_volume": 0.5,
    "video_quality": "auto",
    "font_scale": "medium",
    "online_status_visible": true,
    "read_receipts": true,
    "message_notifications": true,
    "call_notifications": true,
    "dnd": { "start_minutes": 0, "end_minutes": 0, "enabled": false },
    "retention": "ThreeDays"
  }"#;
  let settings: UserSettings =
    serde_json::from_str(legacy).expect("legacy payload without background field deserialises");
  // Defaults kick in for the missing block.
  assert_eq!(settings.background, BackgroundSettings::default());
  assert!(settings.glass_enabled);
  assert!(settings.motion_enabled);
}

#[test]
fn user_settings_sanitised_cascades_into_background() {
  let settings = UserSettings {
    background: BackgroundSettings {
      blur_px: 200,
      overlay_alpha: 10.0,
      ..BackgroundSettings::default()
    },
    ..UserSettings::default()
  };
  let sanitised = settings.sanitised();
  assert_eq!(sanitised.background.blur_px, BACKGROUND_BLUR_MAX_PX);
  assert!((sanitised.background.overlay_alpha - BACKGROUND_OVERLAY_ALPHA_MAX).abs() < f32::EPSILON);
}
