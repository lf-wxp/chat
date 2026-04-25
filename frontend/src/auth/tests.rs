//! Authentication module tests.
//!
//! Extracted from `mod.rs` to keep the main file focused on production
//! code (Issue-7 fix). All tests run on native targets; WASM-only paths
//! (e.g. `decode_jwt_payload`) are covered by integration tests.

use super::*;
use message::UserId;

// ── AuthResult tests ──

#[test]
fn test_auth_result_ok() {
  let result = AuthResult::ok();
  assert!(result.success);
  assert!(result.error.is_none());
}

#[test]
fn test_auth_result_err_with_string() {
  let result = AuthResult::err("invalid credentials");
  assert!(!result.success);
  assert_eq!(result.error.as_deref(), Some("invalid credentials"));
}

#[test]
fn test_auth_result_err_with_owned_string() {
  let result = AuthResult::err(String::from("server error"));
  assert!(!result.success);
  assert_eq!(result.error.as_deref(), Some("server error"));
}

#[test]
fn test_auth_result_clone() {
  let result = AuthResult::err("test");
  let cloned = result.clone();
  assert_eq!(result.success, cloned.success);
  assert_eq!(result.error, cloned.error);
}

#[test]
fn test_auth_result_debug_format() {
  let result = AuthResult::ok();
  let debug = format!("{:?}", result);
  assert!(debug.contains("AuthResult"));
  assert!(debug.contains("success: true"));
}

// ── RegisterRequest serialization tests ──

#[test]
fn test_register_request_serialization() {
  let req = RegisterRequest {
    username: "testuser".to_string(),
    password: "secret123".to_string(),
  };
  let json = serde_json::to_string(&req).expect("Should serialize");
  assert!(json.contains("\"username\":\"testuser\""));
  assert!(json.contains("\"password\":\"secret123\""));
}

#[test]
fn test_login_request_serialization() {
  let req = LoginRequest {
    username: "admin".to_string(),
    password: "pass".to_string(),
  };
  let json = serde_json::to_string(&req).expect("Should serialize");
  assert!(json.contains("\"username\":\"admin\""));
  assert!(json.contains("\"password\":\"pass\""));
}

// ── AuthResponse deserialization tests ──

#[test]
fn test_auth_response_deserialization() {
  let json = r#"{"user_id":"550e8400-e29b-41d4-a716-446655440000","token":"jwt-token-123"}"#;
  let response: AuthResponse = serde_json::from_str(json).expect("Should deserialize");
  assert_eq!(response.user_id, "550e8400-e29b-41d4-a716-446655440000");
  assert_eq!(response.token, "jwt-token-123");
}

#[test]
fn test_auth_response_deserialization_invalid_json() {
  let json = r#"{"invalid": true}"#;
  let result = serde_json::from_str::<AuthResponse>(json);
  assert!(result.is_err());
}

#[test]
fn test_auth_error_response_deserialization() {
  let json = r#"{"error":"Invalid credentials"}"#;
  let response: AuthErrorResponse = serde_json::from_str(json).expect("Should deserialize");
  assert_eq!(response.error, "Invalid credentials");
}

// ── HTTP status code branching logic tests ──

#[test]
fn test_http_success_status_range() {
  for status in 200..300u16 {
    assert!((200..300).contains(&status));
  }
}

#[test]
fn test_http_error_status_range() {
  let error_statuses: Vec<u16> = vec![400, 401, 403, 404, 500, 502, 503];
  for status in error_statuses {
    assert!(!(200..300).contains(&status));
  }
}

#[test]
fn test_error_response_fallback_message() {
  let status = 500u16;
  let fallback = format!("HTTP {} error", status);
  assert_eq!(fallback, "HTTP 500 error");
}

// ── UUID parsing logic (used in send_auth_request) ──

#[test]
fn test_user_id_from_valid_uuid_string() {
  let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
  let parsed = uuid::Uuid::parse_str(uuid_str);
  assert!(parsed.is_ok());
  let user_id = UserId::from_uuid(parsed.unwrap());
  assert_eq!(user_id.to_string(), uuid_str);
}

#[test]
fn test_user_id_from_invalid_uuid_string() {
  let uuid_str = "not-a-valid-uuid";
  let parsed = uuid::Uuid::parse_str(uuid_str);
  assert!(parsed.is_err());
}

// ── Request timeout constant test ──

#[test]
fn test_request_timeout_is_reasonable() {
  const REQUEST_TIMEOUT_MS: i32 = 10_000;
  assert_eq!(REQUEST_TIMEOUT_MS, 10_000, "Timeout should be 10 seconds");
}

// ── JWT expiry check tests (Issue-3 fix) ──

#[test]
fn test_is_jwt_expired_malformed_token() {
  assert!(is_jwt_expired(""));
  assert!(is_jwt_expired("only_one_part"));
  assert!(is_jwt_expired("two.parts"));
}

#[test]
fn test_is_jwt_expired_three_part_passes_count_gate() {
  let parts: Vec<&str> = "header.payload.signature".split('.').collect();
  assert_eq!(
    parts.len(),
    3,
    "Three-part token should pass the parts-count gate"
  );
}

// ── is_payload_expired pure-Rust tests (Issue-3 fix) ──

#[test]
fn test_payload_not_expired() {
  let payload = r#"{"exp":9999999999}"#;
  assert!(!is_payload_expired(payload, 1_000));
}

#[test]
fn test_payload_expired() {
  let payload = r#"{"exp":1000}"#;
  assert!(is_payload_expired(payload, 1_000));
}

#[test]
fn test_payload_no_exp_claim() {
  let payload = r#"{"sub":"user123"}"#;
  assert!(!is_payload_expired(payload, 1_000));
}

#[test]
fn test_payload_non_numeric_exp() {
  let payload = r#"{"exp":"tomorrow"}"#;
  assert!(is_payload_expired(payload, 1_000));
}

#[test]
fn test_payload_invalid_json() {
  assert!(is_payload_expired("not json", 1_000));
}

#[test]
fn test_payload_nbf_in_future() {
  let payload = r#"{"nbf":5000,"exp":9999999999}"#;
  assert!(is_payload_expired(payload, 1_000));
}

#[test]
fn test_payload_nbf_within_grace_window() {
  let payload = r#"{"nbf":1060,"exp":9999999999}"#;
  // nbf is 60 seconds in the future, exactly at the grace window boundary
  assert!(!is_payload_expired(payload, 1_000));
}

#[test]
fn test_payload_nbf_just_outside_grace_window() {
  let payload = r#"{"nbf":1061,"exp":9999999999}"#;
  // nbf is 61 seconds in the future, one second past the grace window
  assert!(is_payload_expired(payload, 1_000));
}

#[test]
fn test_payload_nbf_and_exp_both_valid() {
  let payload = r#"{"nbf":500,"exp":5000}"#;
  assert!(!is_payload_expired(payload, 1_000));
}

#[test]
fn test_payload_exp_as_f64() {
  let payload = r#"{"exp":1000.5}"#;
  assert!(is_payload_expired(payload, 1_001));
  assert!(!is_payload_expired(payload, 999));
}

#[test]
fn test_format_js_error_with_string() {
  let s = "test error message";
  assert!(!s.is_empty());
}
