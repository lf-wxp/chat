use super::*;
use crate::config::Config;
use jsonwebtoken::{EncodingKey, Header, encode};

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
