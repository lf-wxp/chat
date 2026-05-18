use super::*;

#[test]
fn test_register_user() {
  let store = create_test_store();
  let (user_id, token) = store.register("testuser", "password123").unwrap();

  assert!(!user_id.to_string().is_empty());
  assert!(!token.is_empty());

  let user = store.get_user(&user_id).unwrap();
  assert_eq!(user.username, "testuser");
  assert_eq!(user.status, UserStatus::Online);
}

#[test]
fn test_register_duplicate_username() {
  let store = create_test_store();
  store.register("testuser", "password123").unwrap();

  let result = store.register("testuser", "password456");
  assert!(result.is_err());
}

#[test]
fn test_login_user() {
  let store = create_test_store();
  store.register("testuser", "password123").unwrap();

  let (_user_id, token) = store.login("testuser", "password123").unwrap();
  assert!(!token.is_empty());

  let claims = store.verify_token(&token).unwrap();
  assert_eq!(claims.username, "testuser");
}

#[test]
fn test_login_wrong_password() {
  let store = create_test_store();
  store.register("testuser", "password123").unwrap();

  let result = store.login("testuser", "wrongpassword");
  assert!(result.is_err());
}

#[test]
fn test_authenticate_with_token() {
  let store = create_test_store();
  let (_, token) = store.register("testuser", "password123").unwrap();

  let auth_success = store.authenticate_with_token(&token).unwrap();
  assert_eq!(auth_success.username, "testuser");
}

#[test]
fn test_single_device_login() {
  let store = create_test_store();
  let _ = store.register("testuser", "password123").unwrap();

  // First login
  let (_, token1) = store.login("testuser", "password123").unwrap();

  // First token should work
  let auth1 = store.authenticate_with_token(&token1);
  assert!(auth1.is_ok());

  // Login again (simulating another device)
  let (_, token2) = store.login("testuser", "password123").unwrap();

  // Old token should now be invalid
  let auth1_again = store.authenticate_with_token(&token1);
  assert!(auth1_again.is_err());

  // New token should work
  let auth2 = store.authenticate_with_token(&token2);
  assert!(auth2.is_ok());
}

#[test]
fn test_logout() {
  let store = create_test_store();
  let (user_id, _) = store.register("testuser", "password123").unwrap();

  store.logout(&user_id);

  let user = store.get_user(&user_id).unwrap();
  assert_eq!(user.status, UserStatus::Offline);
}

#[test]
fn test_update_status() {
  let store = create_test_store();
  let (user_id, _) = store.register("testuser", "password123").unwrap();

  let change = store.update_status(&user_id, UserStatus::Busy).unwrap();
  assert_eq!(change.status, UserStatus::Busy);

  let user = store.get_user(&user_id).unwrap();
  assert_eq!(user.status, UserStatus::Busy);
}

#[test]
fn test_get_online_users() {
  let store = create_test_store();
  store.register("user1", "password123").unwrap();
  store.register("user2", "password123").unwrap();

  let online = store.get_online_users();
  assert_eq!(online.len(), 2);
}

#[test]
fn test_login_nonexistent_user() {
  let store = create_test_store();

  let result = store.login("nonexistent", "password123");
  assert!(result.is_err());
}

#[test]
fn test_get_nonexistent_user() {
  let store = create_test_store();
  let user_id = message::types::UserId::new();

  let result = store.get_user(&user_id);
  assert!(result.is_none());
}

#[test]
fn test_update_status_nonexistent_user() {
  let store = create_test_store();
  let user_id = message::types::UserId::new();

  let result = store.update_status(&user_id, UserStatus::Busy);
  assert!(result.is_none());
}

#[test]
fn test_logout_nonexistent_user() {
  let store = create_test_store();
  let user_id = message::types::UserId::new();

  // Should not panic
  store.logout(&user_id);
}

#[test]
fn test_empty_store_has_no_online_users() {
  let store = create_test_store();

  let online = store.get_online_users();
  assert!(online.is_empty());
}

#[test]
fn test_logout_changes_status_to_offline() {
  let store = create_test_store();
  let (user_id, _) = store.register("testuser", "password123").unwrap();

  // User should be online after registration
  let user = store.get_user(&user_id).unwrap();
  assert_eq!(user.status, UserStatus::Online);

  // After logout, status should be offline
  store.logout(&user_id);
  let user = store.get_user(&user_id).unwrap();
  assert_eq!(user.status, UserStatus::Offline);

  // User should not be in online users list
  let online = store.get_online_users();
  assert!(online.is_empty());
}

#[test]
fn test_update_status_to_away() {
  let store = create_test_store();
  let (user_id, _) = store.register("testuser", "password123").unwrap();

  // Update to away
  let change = store.update_status(&user_id, UserStatus::Away).unwrap();
  assert_eq!(change.status, UserStatus::Away);

  // Verify user has away status
  let user = store.get_user(&user_id).unwrap();
  assert_eq!(user.status, UserStatus::Away);
}

#[test]
fn test_multiple_status_changes() {
  let store = create_test_store();
  let (user_id, _) = store.register("testuser", "password123").unwrap();

  // Online -> Busy
  let change = store.update_status(&user_id, UserStatus::Busy).unwrap();
  assert_eq!(change.status, UserStatus::Busy);

  // Busy -> Away
  let change = store.update_status(&user_id, UserStatus::Away).unwrap();
  assert_eq!(change.status, UserStatus::Away);

  // Away -> Online
  let change = store.update_status(&user_id, UserStatus::Online).unwrap();
  assert_eq!(change.status, UserStatus::Online);
}

// ── G28: nickname persistence on the global User table ──

#[test]
fn test_set_nickname_writes_to_user_session() {
  let store = create_test_store();
  let (user_id, _) = store.register("testuser", "password123").unwrap();

  // Default nickname mirrors the username.
  let user = store.get_user(&user_id).unwrap();
  assert_eq!(user.nickname, "testuser");

  // A real change returns `true` and the stored nickname updates.
  let changed = store.set_nickname(&user_id, "Cool Nick");
  assert!(changed);

  let user = store.get_user(&user_id).unwrap();
  assert_eq!(user.nickname, "Cool Nick");
}

#[test]
fn test_set_nickname_idempotent_when_unchanged() {
  // The function reports `false` when the new value equals the
  // current one — callers (e.g. the WS handler) can short-circuit
  // any broadcasts on this signal.
  let store = create_test_store();
  let (user_id, _) = store.register("testuser", "password123").unwrap();

  store.set_nickname(&user_id, "Same");
  let second = store.set_nickname(&user_id, "Same");
  assert!(!second);
}

#[test]
fn test_set_nickname_missing_user_is_noop() {
  let store = create_test_store();
  let user_id = message::types::UserId::new();
  let changed = store.set_nickname(&user_id, "Anything");
  assert!(!changed);
}

#[test]
fn test_set_nickname_survives_reauthenticate_with_token() {
  // G28 cross-reload contract: after a nickname change, a fresh
  // `authenticate_with_token` returns the new nickname in its
  // `AuthSuccess` — without this, the client would receive
  // `nickname = username` on every reload and stomp the local
  // mirror.
  let store = create_test_store();
  let (user_id, token) = store.register("testuser", "password123").unwrap();

  store.set_nickname(&user_id, "Reloaded Nick");

  let auth_success = store
    .authenticate_with_token(&token)
    .expect("token should still validate");
  assert_eq!(auth_success.nickname, "Reloaded Nick");
  // Username is unchanged — confirms we did not accidentally
  // alias the two fields.
  assert_eq!(auth_success.username, "testuser");
}
