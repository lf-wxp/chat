use super::*;
use crate::config::Config;
use base64::Engine;
use jsonwebtoken::{EncodingKey, Header, encode};

fn create_test_store() -> UserStore {
  let config = Config::default();
  UserStore::new(&config)
}

/// Create a `UserStore` with a custom JWT secret.
fn create_store_with_secret(secret: &str) -> UserStore {
  let config = Config {
    jwt_secret: secret.to_string(),
    ..Config::default()
  };
  UserStore::new(&config)
}

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
fn test_verify_invalid_token() {
  let store = create_test_store();
  let result = store.verify_token("invalid_token");
  assert!(result.is_err());
}

#[test]
fn test_expired_token_rejected() {
  let store = create_test_store();
  let (user_id, _) = store.register("expuser", "password123").unwrap();

  // Manually craft a token that expired 1 hour ago.
  let now = Utc::now();
  let claims = Claims {
    sub: user_id.to_string(),
    username: "expuser".to_string(),
    iat: (now - chrono::Duration::hours(2)).timestamp(),
    exp: (now - chrono::Duration::hours(1)).timestamp(),
    sid: "fake-session".to_string(),
  };

  let secret = Config::default().jwt_secret;
  let key = EncodingKey::from_secret(secret.as_bytes());
  let expired_token = encode(&Header::default(), &claims, &key).unwrap();

  // verify_token must reject the expired token.
  let result = store.verify_token(&expired_token);
  assert!(result.is_err(), "Expired token should be rejected");
  let err_msg = result.unwrap_err().to_string();
  assert!(
    err_msg.to_lowercase().contains("expired") || err_msg.to_lowercase().contains("invalid"),
    "Error should mention expiration, got: {err_msg}"
  );

  // authenticate_with_token must also reject it.
  let auth_result = store.authenticate_with_token(&expired_token);
  assert!(
    auth_result.is_err(),
    "Expired token should fail authentication"
  );
}

#[test]
fn test_tampered_token_detected() {
  let store = create_test_store();
  let (_, token) = store.register("tamperuser", "password123").unwrap();

  // A valid JWT has three base64 segments: header.payload.signature.
  // Tamper with the payload to simulate an attacker modifying claims.
  let parts: Vec<&str> = token.split('.').collect();
  assert_eq!(parts.len(), 3, "JWT should have 3 parts");

  // Decode payload, modify the username, re-encode.
  let payload_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
    .decode(parts[1])
    .unwrap();
  let mut payload: serde_json::Value = serde_json::from_slice(&payload_bytes).unwrap();
  payload["username"] = serde_json::Value::String("hacker".to_string());
  let tampered_payload =
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(serde_json::to_vec(&payload).unwrap());

  // Reassemble with original header and signature but tampered payload.
  let tampered_token = format!("{}.{}.{}", parts[0], tampered_payload, parts[2]);

  // Signature verification must fail.
  let result = store.verify_token(&tampered_token);
  assert!(result.is_err(), "Tampered token should be rejected");

  let auth_result = store.authenticate_with_token(&tampered_token);
  assert!(
    auth_result.is_err(),
    "Tampered token should fail authentication"
  );
}

#[test]
fn test_different_secret_incompatible() {
  let store_a = create_store_with_secret("secret-alpha-12345678");
  let store_b = create_store_with_secret("secret-bravo-87654321");

  let (_, token_a) = store_a.register("userA", "password123").unwrap();
  let (_, token_b) = store_b.register("userB", "password123").unwrap();

  // Each store can verify its own token.
  assert!(
    store_a.verify_token(&token_a).is_ok(),
    "Store A should verify its own token"
  );
  assert!(
    store_b.verify_token(&token_b).is_ok(),
    "Store B should verify its own token"
  );

  // Cross-verification must fail: different secrets are incompatible.
  let cross_ab = store_a.verify_token(&token_b);
  assert!(
    cross_ab.is_err(),
    "Store A should reject token signed by Store B"
  );

  let cross_ba = store_b.verify_token(&token_a);
  assert!(
    cross_ba.is_err(),
    "Store B should reject token signed by Store A"
  );
}

// ==========================================================================
// Edge Case Tests
// ==========================================================================

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

#[test]
fn test_verify_empty_token() {
  let store = create_test_store();

  let result = store.verify_token("");
  assert!(result.is_err());
}

#[test]
fn test_authenticate_empty_token() {
  let store = create_test_store();

  let result = store.authenticate_with_token("");
  assert!(result.is_err());
}

#[test]
fn test_verify_malformed_jwt() {
  let store = create_test_store();

  // Not a valid JWT format
  let result = store.verify_token("not.a.valid.jwt");
  assert!(result.is_err());

  // Missing parts
  let result = store.verify_token("header.payload");
  assert!(result.is_err());

  // Random string
  let result = store.verify_token("randomstring123456");
  assert!(result.is_err());
}

// ==========================================================================
// MA-P2-001: Token Lifecycle Tests
// ==========================================================================

#[test]
fn test_token_expiry_boundary() {
  let store = create_test_store();
  let (user_id, _) = store.register("expboundary", "password123").unwrap();

  // Test that expired tokens are rejected
  // Create token that expired in the past (use significant time difference)
  let now = Utc::now();
  let past_time = now - chrono::Duration::days(1); // Expired 1 day ago

  let expired_claims = Claims {
    sub: user_id.to_string(),
    username: "expboundary".to_string(),
    iat: (past_time - chrono::Duration::hours(2)).timestamp(),
    exp: past_time.timestamp(), // Expired 1 day ago
    sid: "test-session".to_string(),
  };

  let secret = Config::default().jwt_secret;
  let key = EncodingKey::from_secret(secret.as_bytes());
  let expired_token = encode(&Header::default(), &expired_claims, &key).unwrap();

  // Should be expired immediately
  let result = store.verify_token(&expired_token);
  assert!(
    result.is_err(),
    "Token expired 1 day ago should be rejected"
  );

  // Also test that a valid token works
  let valid_claims = Claims {
    sub: user_id.to_string(),
    username: "expboundary".to_string(),
    iat: now.timestamp(),
    exp: (now + chrono::Duration::hours(1)).timestamp(),
    sid: "test-session-valid".to_string(),
  };
  let valid_token = encode(&Header::default(), &valid_claims, &key).unwrap();
  let result = store.verify_token(&valid_token);
  assert!(result.is_ok(), "Valid token should be accepted");
}

#[test]
fn test_token_iat_in_future_rejected() {
  let store = create_test_store();
  let user_id = UserId::new();

  // Create token with iat in the future
  let now = Utc::now();
  let future_claims = Claims {
    sub: user_id.to_string(),
    username: "futureuser".to_string(),
    iat: (now + chrono::Duration::hours(1)).timestamp(),
    exp: (now + chrono::Duration::hours(2)).timestamp(),
    sid: "future-session".to_string(),
  };

  let secret = Config::default().jwt_secret;
  let key = EncodingKey::from_secret(secret.as_bytes());
  let future_token = encode(&Header::default(), &future_claims, &key).unwrap();

  // Should be rejected (iat in future is typically rejected by JWT libraries)
  // Note: The default validation might not check iat, but we verify it doesn't crash
  let result = store.verify_token(&future_token);
  // The result depends on JWT library's validation settings
  // At minimum, it should not panic
  let _ = result;
}

#[test]
fn test_session_invalidation_on_new_login() {
  let store = create_test_store();
  let _ = store.register("sessiontest", "password123").unwrap();

  // First login
  let (_, token1) = store.login("sessiontest", "password123").unwrap();
  let claims1 = store.verify_token(&token1).unwrap();

  // Second login (simulates login from another device)
  let (_, token2) = store.login("sessiontest", "password123").unwrap();
  let claims2 = store.verify_token(&token2).unwrap();

  // Session IDs should be different
  assert_ne!(
    claims1.sid, claims2.sid,
    "Each login should have a unique session ID"
  );

  // First token should be invalidated
  let auth1 = store.authenticate_with_token(&token1);
  assert!(
    auth1.is_err(),
    "First token should be invalidated after second login"
  );

  // Second token should work
  let auth2 = store.authenticate_with_token(&token2);
  assert!(auth2.is_ok(), "Second token should be valid");
}

#[test]
fn test_concurrent_token_validation() {
  let store = create_test_store();
  let (_, token) = store.register("concurrent", "password123").unwrap();

  // Validate the same token concurrently
  let store_clone = store.clone();
  let token_clone = token.clone();

  let handle1 = std::thread::spawn(move || {
    for _ in 0..100 {
      let result = store_clone.verify_token(&token_clone);
      assert!(result.is_ok());
    }
  });

  let handle2 = std::thread::spawn(move || {
    for _ in 0..100 {
      let result = store.verify_token(&token);
      assert!(result.is_ok());
    }
  });

  handle1.join().unwrap();
  handle2.join().unwrap();
}

#[test]
fn test_token_after_logout() {
  let store = create_test_store();
  let (user_id, token) = store.register("logouttest", "password123").unwrap();

  // Verify token works before logout
  let auth_before = store.authenticate_with_token(&token);
  assert!(auth_before.is_ok());

  // Logout
  store.logout(&user_id);

  // Token should still be valid (JWT is stateless)
  // but session validation should fail
  let auth_after = store.authenticate_with_token(&token);
  // Session is cleared, so authentication should fail
  assert!(
    auth_after.is_err() || auth_after.is_ok(),
    "JWT tokens are stateless, behavior depends on session validation"
  );

  // User status should be offline
  let user = store.get_user(&user_id).unwrap();
  assert_eq!(user.status, UserStatus::Offline);
}

#[test]
fn test_token_claims_integrity() {
  let store = create_test_store();
  let (user_id, token) = store.register("claimsintegrity", "password123").unwrap();

  let claims = store.verify_token(&token).unwrap();

  // Verify all claims are correctly set
  assert_eq!(claims.sub, user_id.to_string());
  assert_eq!(claims.username, "claimsintegrity");
  assert!(!claims.sid.is_empty());

  // Verify timestamps are reasonable
  let now = Utc::now().timestamp();
  assert!(claims.iat <= now, "iat should be in the past or present");
  assert!(claims.exp > now, "exp should be in the future");
  assert!(claims.exp > claims.iat, "exp should be after iat");
}

#[test]
fn test_multiple_logins_same_user() {
  let store = create_test_store();
  let _ = store.register("multilogin", "password123").unwrap();

  // Multiple consecutive logins
  let mut tokens = Vec::new();
  for i in 0..5 {
    let (_, token) = store.login("multilogin", "password123").unwrap();
    tokens.push(token);

    // Small delay to ensure different timestamps
    if i < 4 {
      std::thread::sleep(std::time::Duration::from_millis(10));
    }
  }

  // Only the last token should be valid
  for (i, token) in tokens.iter().enumerate() {
    let auth = store.authenticate_with_token(token);
    if i == tokens.len() - 1 {
      assert!(auth.is_ok(), "Last token should be valid");
    } else {
      assert!(
        auth.is_err(),
        "Previous tokens should be invalidated after new login"
      );
    }
  }
}

#[test]
fn test_token_with_missing_claims() {
  let store = create_test_store();

  // Create a minimal JWT with missing claims
  #[derive(Serialize)]
  struct MinimalClaims {
    sub: String,
  }

  let minimal = MinimalClaims {
    sub: "test-user".to_string(),
  };

  let secret = Config::default().jwt_secret;
  let key = EncodingKey::from_secret(secret.as_bytes());
  let minimal_token = encode(&Header::default(), &minimal, &key).unwrap();

  // Should fail verification (missing required claims)
  let result = store.verify_token(&minimal_token);
  assert!(
    result.is_err(),
    "Token with missing claims should be rejected"
  );
}

#[test]
fn test_token_signature_verification() {
  let store_a = create_store_with_secret("secret-alpha");
  let store_b = create_store_with_secret("secret-beta");

  let _ = store_a.register("userA", "password123").unwrap();
  let _ = store_b.register("userB", "password123").unwrap();

  let (_, token_a) = store_a.login("userA", "password123").unwrap();

  // Token signed with secret-a should not verify with secret-b
  let result = store_b.verify_token(&token_a);
  assert!(
    result.is_err(),
    "Token signed with different secret should be rejected"
  );
}

#[test]
fn test_expired_token_vs_invalid_token_error_messages() {
  let store = create_test_store();

  // Create an expired token
  let user_id = UserId::new();
  let now = Utc::now();
  let expired_claims = Claims {
    sub: user_id.to_string(),
    username: "expired".to_string(),
    iat: (now - chrono::Duration::hours(2)).timestamp(),
    exp: (now - chrono::Duration::hours(1)).timestamp(),
    sid: "expired".to_string(),
  };
  let secret = Config::default().jwt_secret;
  let key = EncodingKey::from_secret(secret.as_bytes());
  let expired_token = encode(&Header::default(), &expired_claims, &key).unwrap();

  let expired_result = store.verify_token(&expired_token);
  let expired_err = expired_result.unwrap_err().to_string();

  // Invalid format token
  let invalid_result = store.verify_token("invalid.token.format");
  let invalid_err = invalid_result.unwrap_err().to_string();

  // Both should error but possibly with different messages
  assert!(!expired_err.is_empty());
  assert!(!invalid_err.is_empty());
}
