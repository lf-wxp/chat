//! Unit tests for `data_management_helpers`.

use super::*;

#[test]
fn format_bytes_scales() {
  assert_eq!(format_bytes(512), "512 B");
  assert_eq!(format_bytes(2_048), "2.00 KB");
  assert_eq!(format_bytes(5_242_880), "5.00 MB");
  assert_eq!(format_bytes(2 * 1_073_741_824), "2.00 GB");
}

#[test]
fn storage_estimate_handles_missing() {
  assert_eq!(format_storage_estimate(None, "unknown"), "unknown");
  assert_eq!(format_storage_estimate(Some((0, 0)), "unknown"), "0 B");
  assert_eq!(
    format_storage_estimate(Some((1024, 10 * 1024)), "unknown"),
    "1.00 KB / 10.00 KB"
  );
}

#[test]
fn preserved_storage_keys_match_prefix_list() {
  assert!(is_preserved_storage_key("settings_user"));
  assert!(is_preserved_storage_key("settings_theme"));
  assert!(is_preserved_storage_key("theme"));
  assert!(is_preserved_storage_key("locale"));
  assert!(is_preserved_storage_key("blacklist"));
  assert!(is_preserved_storage_key("auth_token"));
  assert!(is_preserved_storage_key("pinned_rooms"));

  // Non-matching keys are considered non-critical and get purged.
  assert!(!is_preserved_storage_key("cache_sticker_v1"));
  assert!(!is_preserved_storage_key("debug_filter"));
  assert!(!is_preserved_storage_key("random"));
  assert!(!is_preserved_storage_key(""));
  // Legacy `user_settings` key is intentionally not preserved: it
  // is one-shot-imported on startup and then dropped (V2-S-4).
  assert!(!is_preserved_storage_key("user_settings"));
}

#[test]
fn timestamped_filename_format() {
  let name = timestamped_filename("chat-export", "json");
  assert!(name.starts_with("chat-export-"));
  assert!(name.ends_with(".json"));
  // YYYY-MM-DD is 10 characters, so the body length is fixed.
  assert_eq!(name.len(), "chat-export-YYYY-MM-DD.json".len());
}
