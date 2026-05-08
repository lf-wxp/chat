//! WASM integration tests for settings persistence.
//!
//! These tests exercise localStorage round-tripping and the reactive
//! `SettingsState` API. They complement the native unit tests in
//! `tests.rs` which cover pure data helpers (serde, sanitisation, DND
//! window math).
//!
//! Run with: `wasm-pack test --firefox --headless` (or `--chrome`).

use leptos::prelude::GetUntracked;
use wasm_bindgen_test::*;

use super::{
  LEGACY_STORAGE_KEY, STORAGE_KEY, SettingsState, UserSettings, load_snapshot, persist_to_storage,
};
use crate::utils;

wasm_bindgen_test_configure!(run_in_browser);

// ── Settings persistence round-trip ──────────────────────────────────

#[wasm_bindgen_test]
fn settings_persist_and_load_round_trip() {
  // Clean slate — remove any stale data from previous test runs.
  utils::remove_from_local_storage(STORAGE_KEY);
  utils::remove_from_local_storage(LEGACY_STORAGE_KEY);

  let settings = UserSettings {
    speaker_volume: 0.42,
    microphone_volume: 0.73,
    font_scale: super::FontScale::Large,
    video_quality: super::VideoQualityPref::High,
    online_status_visible: false,
    read_receipts: false,
    message_notifications: false,
    call_notifications: true,
    dnd: super::DndWindow {
      start_minutes: 1320, // 22:00
      end_minutes: 420,    // 07:00
      enabled: true,
    },
    ..UserSettings::default()
  };

  // Persist and reload.
  persist_to_storage(&settings);
  let loaded = load_snapshot();

  assert!((loaded.speaker_volume - 0.42).abs() < f32::EPSILON);
  assert!((loaded.microphone_volume - 0.73).abs() < f32::EPSILON);
  assert_eq!(loaded.font_scale, super::FontScale::Large);
  assert_eq!(loaded.video_quality, super::VideoQualityPref::High);
  assert!(!loaded.online_status_visible);
  assert!(!loaded.read_receipts);
  assert!(!loaded.message_notifications);
  assert!(loaded.call_notifications);
  assert!(loaded.dnd.enabled);
  assert_eq!(loaded.dnd.start_minutes, 1320);
  assert_eq!(loaded.dnd.end_minutes, 420);

  // Clean up.
  utils::remove_from_local_storage(STORAGE_KEY);
}

#[wasm_bindgen_test]
fn settings_state_update_persists_to_local_storage() {
  utils::remove_from_local_storage(STORAGE_KEY);

  let state = SettingsState::new();

  // Mutate via the reactive API — this should auto-persist.
  state.update(|s| {
    s.speaker_volume = 0.15;
    s.font_scale = super::FontScale::Small;
  });

  // Verify the persisted JSON contains the mutated values.
  let raw = utils::load_from_local_storage(STORAGE_KEY).unwrap_or_default();
  assert!(
    raw.contains("0.15"),
    "persisted JSON should contain 0.15 speaker_volume"
  );
  assert!(
    raw.contains("small"),
    "persisted JSON should contain small font_scale"
  );

  // Clean up.
  utils::remove_from_local_storage(STORAGE_KEY);
}

#[wasm_bindgen_test]
fn settings_saved_tick_increments_on_update() {
  utils::remove_from_local_storage(STORAGE_KEY);

  let state = SettingsState::new();
  let tick_before = state.saved_tick().get_untracked();

  state.update(|s| {
    s.speaker_volume = 0.55;
  });

  let tick_after = state.saved_tick().get_untracked();
  assert_eq!(
    tick_after,
    tick_before.wrapping_add(1),
    "saved_tick should increment once per update"
  );

  // Bump without mutation.
  state.bump_saved();
  let tick_after_bump = state.saved_tick().get_untracked();
  assert_eq!(
    tick_after_bump,
    tick_after.wrapping_add(1),
    "bump_saved should also increment tick"
  );

  utils::remove_from_local_storage(STORAGE_KEY);
}

// ── Legacy migration ─────────────────────────────────────────────────

#[wasm_bindgen_test]
fn legacy_storage_key_migrated_on_load() {
  // Clean slate.
  utils::remove_from_local_storage(STORAGE_KEY);
  utils::remove_from_local_storage(LEGACY_STORAGE_KEY);

  // Write under the legacy key.
  let settings = UserSettings {
    speaker_volume: 0.33,
    ..UserSettings::default()
  };
  let json = serde_json::to_string(&settings).unwrap();
  utils::save_to_local_storage(LEGACY_STORAGE_KEY, &json);

  // Loading should migrate the data.
  let loaded = load_snapshot();
  assert!((loaded.speaker_volume - 0.33).abs() < f32::EPSILON);

  // The new key should now exist.
  assert!(utils::load_from_local_storage(STORAGE_KEY).is_some());

  // The legacy key should have been removed.
  assert!(
    utils::load_from_local_storage(LEGACY_STORAGE_KEY).is_none(),
    "legacy key should be removed after migration"
  );

  utils::remove_from_local_storage(STORAGE_KEY);
}
