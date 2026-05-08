use super::*;
use crate::config::Config;
use base64::Engine;
use jsonwebtoken::{EncodingKey, Header, encode};

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
