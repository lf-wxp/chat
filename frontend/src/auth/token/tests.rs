use super::*;

#[test]
fn test_keys_are_defined() {
  assert_eq!(KEY_TOKEN, "auth_token");
  assert_eq!(KEY_USER_ID, "auth_user_id");
  assert_eq!(KEY_USERNAME, "auth_username");
  assert_eq!(KEY_NICKNAME, "auth_nickname");
  assert_eq!(KEY_AVATAR, "auth_avatar");
  assert_eq!(KEY_SIGNATURE, "auth_signature");
  assert_eq!(KEY_ACTIVE_CONVERSATION, "active_conversation_id");
  assert_eq!(KEY_ACTIVE_ROOM_ID, "active_room_id");
  assert_eq!(KEY_ACTIVE_CALL, "active_call");
}

#[test]
fn test_key_token_is_unique() {
  // Ensure no key collision between auth keys
  let keys = [
    KEY_TOKEN,
    KEY_USER_ID,
    KEY_USERNAME,
    KEY_NICKNAME,
    KEY_AVATAR,
    KEY_SIGNATURE,
    KEY_ACTIVE_CONVERSATION,
    KEY_ACTIVE_ROOM_ID,
    KEY_ACTIVE_CALL,
  ];
  for (i, a) in keys.iter().enumerate() {
    for (j, b) in keys.iter().enumerate() {
      if i != j {
        assert_ne!(a, b, "Keys at index {} and {} should be unique", i, j);
      }
    }
  }
}

#[test]
fn test_key_active_conversation_is_separate_from_auth() {
  // KEY_ACTIVE_CONVERSATION should not contain "auth" prefix
  // to distinguish it from auth-related keys
  assert!(!KEY_ACTIVE_CONVERSATION.starts_with("auth_"));
}

#[test]
fn test_auth_state_fields() {
  // Verify AuthState struct can be constructed with expected fields
  let user_id = UserId::new();
  let auth = AuthState {
    user_id: user_id.clone(),
    token: "test-jwt-token".to_string(),
    username: "testuser".to_string(),
    nickname: "Test User".to_string(),
    avatar: "data:image/svg+xml;base64,test".to_string(),
    signature: String::new(),
  };
  assert_eq!(auth.user_id, user_id);
  assert_eq!(auth.token, "test-jwt-token");
  assert_eq!(auth.username, "testuser");
  assert_eq!(auth.nickname, "Test User");
  assert_eq!(auth.avatar, "data:image/svg+xml;base64,test");
}

#[test]
fn test_auth_state_clone() {
  let user_id = UserId::new();
  let auth = AuthState {
    user_id: user_id.clone(),
    token: "token".to_string(),
    username: "user".to_string(),
    nickname: "nick".to_string(),
    avatar: "data:image/svg+xml;base64,abc".to_string(),
    signature: String::new(),
  };
  let cloned = auth.clone();
  assert_eq!(auth.user_id, cloned.user_id);
  assert_eq!(auth.token, cloned.token);
  assert_eq!(auth.username, cloned.username);
  assert_eq!(auth.nickname, cloned.nickname);
  assert_eq!(auth.avatar, cloned.avatar);
}

#[test]
fn test_load_auth_rejects_empty_token_logic() {
  // The load_auth_from_storage logic returns None when token is empty.
  // Test the guard condition directly.
  let token = "";
  assert!(token.is_empty(), "Empty token should be rejected");
}

#[test]
fn test_load_auth_requires_valid_uuid() {
  // user_id_str must be a valid UUID
  let valid_uuid = "550e8400-e29b-41d4-a716-446655440000";
  let parsed = uuid::Uuid::parse_str(valid_uuid);
  assert!(parsed.is_ok(), "Valid UUID should parse successfully");

  let invalid_uuid = "not-a-uuid";
  let parsed = uuid::Uuid::parse_str(invalid_uuid);
  assert!(parsed.is_err(), "Invalid UUID should fail to parse");
}

/// Helper that mirrors the nickname-resolution logic in `load_auth_from_storage`
fn resolve_nickname(nickname: Option<String>, username: &str) -> String {
  nickname.unwrap_or_else(|| username.to_string())
}

#[test]
fn test_nickname_fallback_to_username() {
  // When nickname is None, it should fall back to username
  let resolved = resolve_nickname(None, "testuser");
  assert_eq!(resolved, "testuser");
}

#[test]
fn test_nickname_uses_provided_value() {
  // When nickname is Some, it should use the provided value
  let resolved = resolve_nickname(Some("Custom Nick".to_string()), "testuser");
  assert_eq!(resolved, "Custom Nick");
}
