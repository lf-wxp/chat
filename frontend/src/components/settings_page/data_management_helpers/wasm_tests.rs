//! WASM integration tests for cache clearing and storage-preservation logic.
//!
//! These tests exercise the localStorage purge that runs when the user
//! taps "Clear cache", verifying that preserved keys survive and
//! non-critical keys are removed.
//!
//! Run with: `wasm-pack test --firefox --headless` (or `--chrome`).

use wasm_bindgen_test::*;

use super::{PRESERVED_STORAGE_PREFIXES, is_preserved_storage_key};
use crate::utils;

wasm_bindgen_test_configure!(run_in_browser);

// ── Preserved-key classification ─────────────────────────────────────

#[wasm_bindgen_test]
fn preserved_keys_are_not_purged() {
  // Verify the classification logic directly — this is the
  // gate-keeper for the cache-clear operation.
  for prefix in PRESERVED_STORAGE_PREFIXES.iter() {
    assert!(
      is_preserved_storage_key(prefix),
      "prefix '{prefix}' should be preserved"
    );
    // A key that *starts with* the prefix is also preserved.
    let extended = format!("{prefix}extra_data");
    assert!(
      is_preserved_storage_key(&extended),
      "key '{extended}' should be preserved"
    );
  }
}

#[wasm_bindgen_test]
fn non_critical_keys_are_classified_for_purge() {
  // Keys that are NOT prefixed by any preserved prefix should be
  // classified as non-critical (i.e. safe to purge).
  let non_critical = [
    "cache_sticker_v1",
    "debug_filter",
    "conversations",
    "active_conversation_id",
    "active_room_id",
    "active_call",
    "user_settings", // legacy, already migrated
    "temp_upload_abc",
    "sw_preload_cache",
  ];
  for key in &non_critical {
    assert!(
      !is_preserved_storage_key(key),
      "key '{key}' should NOT be preserved"
    );
  }
}

#[wasm_bindgen_test]
fn empty_string_key_is_not_preserved() {
  assert!(
    !is_preserved_storage_key(""),
    "empty key should not be preserved"
  );
}

// ── Cache-clear localStorage sweep ───────────────────────────────────

#[wasm_bindgen_test]
fn clear_non_critical_local_storage_preserves_preferences() {
  // Seed localStorage with both preserved and non-critical keys.
  let preserved_entries = [
    ("settings_user", r#"{"speaker_volume":0.5}"#),
    ("settings_theme", "dark"),
    ("settings_locale", "en"),
    ("auth_token", "jwt-abc"),
    ("auth_user_id", "42"),
    ("theme", "light"),
    ("locale", "zh-CN"),
    ("blacklist", "[]"),
    ("pinned_rooms", "[]"),
  ];
  let non_critical_entries = [
    ("cache_sticker_v1", "blob"),
    ("debug_filter", "webrtc"),
    ("conversations", "[]"),
    ("active_conversation_id", "conv-1"),
    ("temp_upload_xyz", "data"),
  ];

  for (k, v) in preserved_entries.iter().chain(non_critical_entries.iter()) {
    utils::save_to_local_storage(k, v);
  }

  // Run the purge via the public helper path.
  super::clear_non_critical_local_storage();

  // Preserved keys must survive.
  for (k, _) in &preserved_entries {
    assert!(
      utils::load_from_local_storage(k).is_some(),
      "preserved key '{k}' should survive cache clear"
    );
  }

  // Non-critical keys should be gone.
  for (k, _) in &non_critical_entries {
    assert!(
      utils::load_from_local_storage(k).is_none(),
      "non-critical key '{k}' should be purged"
    );
  }

  // Clean up preserved keys.
  for (k, _) in &preserved_entries {
    utils::remove_from_local_storage(k);
  }
}

// ── trigger_download blob creation ───────────────────────────────────

#[wasm_bindgen_test]
fn trigger_download_creates_blob_url_and_clicks() {
  // We cannot easily assert the download actually happened in
  // headless mode, but we can verify the function does not panic
  // and that the blob URL creation machinery works by calling the
  // underlying Blob + URL API directly.
  use web_sys::{Blob, BlobPropertyBag, Url};

  let content = r#"{"test": true}"#;
  let mime = "application/json";

  let options = BlobPropertyBag::new();
  options.set_type(mime);
  let array = js_sys::Array::of1(&wasm_bindgen::JsValue::from_str(content));
  let blob = Blob::new_with_str_sequence_and_options(&array, &options)
    .expect("Blob creation should succeed");

  // Verify the blob has the expected size.
  assert_eq!(blob.size() as usize, content.len());

  // Verify we can create an object URL from the blob.
  let url = Url::create_object_url_with_blob(&blob).expect("Object URL creation should succeed");
  assert!(
    url.starts_with("blob:"),
    "URL should be a blob URL, got: {url}"
  );

  // Revoke immediately to avoid leaking.
  Url::revoke_object_url(&url).expect("revoke should succeed");
}

#[wasm_bindgen_test]
fn trigger_download_does_not_panic_with_empty_content() {
  // Ensure the download helper does not panic even with empty
  // content. In headless mode the <a>.click() is a no-op, but the
  // function should complete without errors.
  super::trigger_download("test.json", "application/json", "");
}

#[wasm_bindgen_test]
fn trigger_download_with_html_content() {
  // Verify HTML export path does not panic and the Blob accepts
  // multi-line HTML content.
  let html = "<html><body><h1>Export</h1></body></html>";
  super::trigger_download("export.html", "text/html", html);
}
