//! WASM integration tests for the frontend crate.
//!
//! These tests run in a real WASM runtime (via `wasm-pack test --node`)
//! and exercise browser-API-dependent code paths that cannot be covered
//! by native `cargo test`.
//!
//! Minimum test set required by Issue-14 (review-task-14.md §6.2):
//! - localStorage roundtrip
//! - Identicon SVG validity
//! - JWT expiry decode (browser `atob` path)
//! - Identicon data URI format
//! - URL encoding correctness
//! - Auth storage clear completeness

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ============================================================================
// localStorage Roundtrip Tests
// ============================================================================

/// Verify that save → load → remove works correctly with real localStorage.
#[wasm_bindgen_test]
fn test_local_storage_roundtrip() {
  use chat_frontend::utils;

  let key = "__wasm_test_roundtrip";
  let value = "hello_wasm_test_123";

  // Clean up any leftover from previous runs
  utils::remove_from_local_storage(key);
  assert!(
    utils::load_from_local_storage(key).is_none(),
    "Key should not exist before save"
  );

  // Save
  utils::save_to_local_storage(key, value);

  // Load
  let loaded = utils::load_from_local_storage(key);
  assert_eq!(
    loaded.as_deref(),
    Some(value),
    "Should load back the saved value"
  );

  // Remove
  utils::remove_from_local_storage(key);
  assert!(
    utils::load_from_local_storage(key).is_none(),
    "Key should be gone after remove"
  );
}

/// Verify that loading a non-existent key returns None.
#[wasm_bindgen_test]
fn test_local_storage_missing_key_returns_none() {
  use chat_frontend::utils;

  let result = utils::load_from_local_storage("__wasm_test_nonexistent_key_xyz");
  assert!(result.is_none(), "Missing key should return None");
}

/// Verify that saving an empty string is stored and retrievable.
#[wasm_bindgen_test]
fn test_local_storage_empty_value() {
  use chat_frontend::utils;

  let key = "__wasm_test_empty_value";
  utils::save_to_local_storage(key, "");
  let loaded = utils::load_from_local_storage(key);
  assert_eq!(loaded.as_deref(), Some(""), "Empty string should be stored");
  utils::remove_from_local_storage(key);
}

/// Verify that Unicode values survive the localStorage roundtrip.
#[wasm_bindgen_test]
fn test_local_storage_unicode_value() {
  use chat_frontend::utils;

  let key = "__wasm_test_unicode";
  let value = "用户名_🎉_émojis";
  utils::save_to_local_storage(key, value);
  let loaded = utils::load_from_local_storage(key);
  assert_eq!(
    loaded.as_deref(),
    Some(value),
    "Unicode should survive roundtrip"
  );
  utils::remove_from_local_storage(key);
}

// ============================================================================
// Identicon SVG Validity Tests
// ============================================================================

/// Verify that the identicon SVG is well-formed XML.
#[wasm_bindgen_test]
fn test_identicon_svg_is_valid_xml() {
  let svg = chat_frontend::identicon::generate_identicon_svg("wasm_test_user");
  assert!(svg.starts_with("<svg"), "Should start with <svg");
  assert!(svg.contains("</svg>"), "Should end with </svg>");
  assert!(
    svg.contains("xmlns=\"http://www.w3.org/2000/svg\""),
    "Should have SVG namespace"
  );
  assert!(svg.contains("viewBox="), "Should have viewBox attribute");
}

/// Verify that the identicon data URI is properly formatted.
#[wasm_bindgen_test]
fn test_identicon_data_uri_is_valid() {
  let uri = chat_frontend::identicon::generate_identicon_data_uri("wasm_test_user");
  assert!(
    uri.starts_with("data:image/svg+xml;charset=utf-8,"),
    "Should have correct data URI prefix, got: {}",
    &uri[..uri.len().min(50)]
  );
  // The URI should not contain raw < or > (they must be percent-encoded)
  let after_prefix = &uri["data:image/svg+xml;charset=utf-8,".len()..];
  assert!(
    !after_prefix.contains('<'),
    "Raw < should be percent-encoded in data URI"
  );
  assert!(
    !after_prefix.contains('>'),
    "Raw > should be percent-encoded in data URI"
  );
}

/// Verify determinism: same input always produces the same identicon.
#[wasm_bindgen_test]
fn test_identicon_is_deterministic() {
  let uri1 = chat_frontend::identicon::generate_identicon_data_uri("determinism_test");
  let uri2 = chat_frontend::identicon::generate_identicon_data_uri("determinism_test");
  assert_eq!(uri1, uri2, "Same input should produce identical output");
}

/// Verify that different usernames produce different identicons.
#[wasm_bindgen_test]
fn test_identicon_different_usernames() {
  let uri1 = chat_frontend::identicon::generate_identicon_data_uri("alice");
  let uri2 = chat_frontend::identicon::generate_identicon_data_uri("bob");
  assert_ne!(
    uri1, uri2,
    "Different usernames should produce different identicons"
  );
}

/// Verify that the identicon SVG contains colored rect elements.
#[wasm_bindgen_test]
fn test_identicon_svg_has_grid_rects() {
  let svg = chat_frontend::identicon::generate_identicon_svg("grid_test");
  // The SVG should contain at least one <rect element for the grid
  // (background rect + at least some filled cells)
  assert!(
    svg.contains("<rect"),
    "SVG should contain rect elements for the grid"
  );
  // Should contain hsl color values
  assert!(svg.contains("hsl("), "SVG should use HSL colors");
}

// ============================================================================
// Auth Token Persistence Tests (real localStorage)
// ============================================================================

/// Verify that save_active_room_id → load_active_room_id works.
#[wasm_bindgen_test]
fn test_active_room_id_roundtrip() {
  use chat_frontend::auth;

  let room_id = "550e8400-e29b-41d4-a716-446655440000";

  // Save
  auth::save_active_room_id(Some(room_id));

  // Load
  let loaded = auth::load_active_room_id();
  assert_eq!(
    loaded.as_deref(),
    Some(room_id),
    "Should load back the saved room ID"
  );

  // Clear
  auth::save_active_room_id(None);
  assert!(
    auth::load_active_room_id().is_none(),
    "Room ID should be None after clear"
  );
}

/// Verify that save_active_call → load_active_call works.
#[wasm_bindgen_test]
fn test_active_call_roundtrip() {
  use chat_frontend::auth;

  let call_json = r#"{"room_id":"test-room","call_type":"audio"}"#;

  auth::save_active_call(Some(call_json));
  let loaded = auth::load_active_call();
  assert_eq!(
    loaded.as_deref(),
    Some(call_json),
    "Should load back the saved call JSON"
  );

  auth::save_active_call(None);
  assert!(
    auth::load_active_call().is_none(),
    "Call should be None after clear"
  );
}

/// Verify that clear_auth_storage removes all 8 keys.
#[wasm_bindgen_test]
fn test_clear_auth_storage_removes_all_keys() {
  use chat_frontend::auth;
  use chat_frontend::utils;

  // Pre-populate some keys that clear_auth_storage should remove
  utils::save_to_local_storage("auth_token", "test-token");
  utils::save_to_local_storage("auth_user_id", "test-uid");
  utils::save_to_local_storage("auth_username", "testuser");
  utils::save_to_local_storage("auth_nickname", "Test");
  utils::save_to_local_storage("auth_avatar", "data:image/svg+xml;...");
  utils::save_to_local_storage("auth_signature", "Hello world");
  utils::save_to_local_storage("active_conversation_id", "conv-1");
  utils::save_to_local_storage("active_room_id", "room-1");
  utils::save_to_local_storage("active_call", "{}");

  // Clear
  auth::clear_auth_storage();

  // Verify all keys are removed
  assert!(utils::load_from_local_storage("auth_token").is_none());
  assert!(utils::load_from_local_storage("auth_user_id").is_none());
  assert!(utils::load_from_local_storage("auth_username").is_none());
  assert!(utils::load_from_local_storage("auth_nickname").is_none());
  assert!(utils::load_from_local_storage("auth_avatar").is_none());
  assert!(utils::load_from_local_storage("auth_signature").is_none());
  assert!(utils::load_from_local_storage("active_conversation_id").is_none());
  assert!(utils::load_from_local_storage("active_room_id").is_none());
  assert!(utils::load_from_local_storage("active_call").is_none());
}

// ============================================================================
// Auth State Persistence Tests (real localStorage end-to-end)
// ============================================================================

/// Verify that save_auth_to_storage → load_auth_from_storage roundtrip
/// works with real localStorage, including Identicon avatar generation.
#[wasm_bindgen_test]
fn test_auth_state_full_roundtrip() {
  use chat_frontend::auth;
  use chat_frontend::state::AuthState;

  // Clean up from any previous test run
  auth::clear_auth_storage();

  let user_id = message::UserId::new();
  let auth_state = AuthState {
    user_id: user_id.clone(),
    token: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test.sig".to_string(),
    username: "wasm_roundtrip_user".to_string(),
    nickname: "WASM Test".to_string(),
    avatar: String::new(),
    signature: String::new(),
  };

  // Save
  auth::save_auth_to_storage(&auth_state);

  // Load
  let loaded = auth::load_auth_from_storage();
  assert!(loaded.is_some(), "Should load back saved auth state");
  let loaded = loaded.unwrap();
  assert_eq!(loaded.user_id, user_id, "user_id should match");
  assert_eq!(loaded.token, auth_state.token, "token should match");
  assert_eq!(
    loaded.username, auth_state.username,
    "username should match"
  );
  // nickname falls back to username when stored nickname is read from localStorage
  assert_eq!(
    loaded.nickname, auth_state.nickname,
    "nickname should match"
  );

  // Avatar should also have been persisted (Identicon generated from username)
  let avatar = auth::load_avatar_from_storage();
  assert!(avatar.is_some(), "Avatar should be persisted");
  let avatar = avatar.unwrap();
  assert!(
    avatar.starts_with("data:image/svg+xml"),
    "Avatar should be an SVG data URI"
  );

  // Verify the avatar matches a fresh generation for the same username
  let fresh_avatar = chat_frontend::identicon::generate_identicon_data_uri("wasm_roundtrip_user");
  assert_eq!(
    avatar, fresh_avatar,
    "Persisted avatar should match freshly generated identicon"
  );

  // Clean up
  auth::clear_auth_storage();
}

/// Verify that load_auth_from_storage returns None when token is empty.
#[wasm_bindgen_test]
fn test_auth_load_rejects_empty_token() {
  use chat_frontend::auth;
  use chat_frontend::utils;

  // Manually set an empty token
  auth::clear_auth_storage();
  utils::save_to_local_storage("auth_token", "");
  utils::save_to_local_storage("auth_user_id", "550e8400-e29b-41d4-a716-446655440000");
  utils::save_to_local_storage("auth_username", "test");

  let loaded = auth::load_auth_from_storage();
  assert!(
    loaded.is_none(),
    "Should reject auth state with empty token"
  );

  auth::clear_auth_storage();
}

/// Verify that load_auth_from_storage returns None when user_id is invalid.
#[wasm_bindgen_test]
fn test_auth_load_rejects_invalid_uuid() {
  use chat_frontend::auth;
  use chat_frontend::utils;

  auth::clear_auth_storage();
  utils::save_to_local_storage("auth_token", "valid-token");
  utils::save_to_local_storage("auth_user_id", "not-a-uuid");
  utils::save_to_local_storage("auth_username", "test");

  let loaded = auth::load_auth_from_storage();
  assert!(
    loaded.is_none(),
    "Should reject auth state with invalid UUID"
  );

  auth::clear_auth_storage();
}

// ============================================================================
// JWT Expiry Tests (browser atob path)
// ============================================================================

/// Verify that a token with a future `exp` is NOT expired.
#[wasm_bindgen_test]
fn test_is_jwt_expired_future_token() {
  use chat_frontend::auth;

  // Header: {"alg":"none","typ":"JWT"}
  // Payload: {"exp":9999999999} (far future)
  let token = "eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.eyJleHAiOjk5OTk5OTk5OTl9.sig";
  assert!(
    !auth::is_jwt_expired(token),
    "Future token should not be expired"
  );
}

/// Verify that a token with a past `exp` IS expired.
#[wasm_bindgen_test]
fn test_is_jwt_expired_past_token() {
  use chat_frontend::auth;

  // Payload: {"exp":1} (far past)
  let token = "eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.eyJleHAiOjF9.sig";
  assert!(auth::is_jwt_expired(token), "Past token should be expired");
}

/// Verify that a token without `exp` is treated as non-expiring.
///
/// The client defers to the server for tokens that omit the `exp` claim.
#[wasm_bindgen_test]
fn test_is_jwt_expired_missing_exp() {
  use chat_frontend::auth;

  // Payload: {"sub":"user"} (no exp claim)
  let token = "eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.eyJzdWIiOiJ1c2VyIn0.sig";
  assert!(
    !auth::is_jwt_expired(token),
    "Token without exp should be treated as non-expiring (server will validate)"
  );
}

/// Verify that a token with non-numeric `exp` is treated as expired.
#[wasm_bindgen_test]
fn test_is_jwt_expired_non_numeric_exp() {
  use chat_frontend::auth;

  // Payload: {"exp":"not-a-number"}
  let token = "eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.eyJleHAiOiJub3QtYS1udW1iZXIifQ.sig";
  assert!(
    auth::is_jwt_expired(token),
    "Token with non-numeric exp should be treated as expired"
  );
}
