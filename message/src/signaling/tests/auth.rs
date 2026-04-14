//! Authentication message tests.

use super::*;

#[test]
fn test_token_auth_roundtrip() {
  let msg = TokenAuth {
    token: "test_token_123".to_string(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: TokenAuth = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_auth_success_roundtrip() {
  let msg = AuthSuccess {
    user_id: UserId::new(),
    username: "alice".to_string(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: AuthSuccess = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_auth_failure_roundtrip() {
  let msg = AuthFailure {
    reason: "Invalid token".to_string(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: AuthFailure = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_user_logout_roundtrip() {
  let msg = UserLogout {};
  let encoded = bitcode::encode(&msg);
  let decoded: UserLogout = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_ping_roundtrip() {
  let msg = Ping {};
  let encoded = bitcode::encode(&msg);
  let decoded: Ping = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_pong_roundtrip() {
  let msg = Pong {};
  let encoded = bitcode::encode(&msg);
  let decoded: Pong = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_session_invalidated_roundtrip() {
  let msg = SessionInvalidated {};
  let encoded = bitcode::encode(&msg);
  let decoded: SessionInvalidated = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_session_invalidated_discriminator() {
  let msg = SignalingMessage::SessionInvalidated(SessionInvalidated {});
  assert_eq!(msg.discriminator(), 0x07);
}

#[test]
fn test_error_response_signaling_roundtrip() {
  let error_resp = crate::ErrorResponse::new(SIG001, "Connection failed", "trace-err-001");
  let msg = SignalingMessage::ErrorResponse(error_resp);
  let encoded = bitcode::encode(&msg);
  let decoded: SignalingMessage = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_signaling_message_user_logout_roundtrip() {
  let msg = SignalingMessage::UserLogout(UserLogout {});
  let encoded = bitcode::encode(&msg);
  let decoded: SignalingMessage = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_signaling_message_ping_roundtrip() {
  let msg = SignalingMessage::Ping(Ping {});
  let encoded = bitcode::encode(&msg);
  let decoded: SignalingMessage = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_signaling_message_pong_roundtrip() {
  let msg = SignalingMessage::Pong(Pong {});
  let encoded = bitcode::encode(&msg);
  let decoded: SignalingMessage = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_discriminator_auth_messages() {
  assert_eq!(
    SignalingMessage::TokenAuth(TokenAuth { token: "t".into() }).discriminator(),
    TOKEN_AUTH
  );
  assert_eq!(
    SignalingMessage::AuthSuccess(AuthSuccess {
      user_id: UserId::new(),
      username: "u".into()
    })
    .discriminator(),
    AUTH_SUCCESS
  );
  assert_eq!(
    SignalingMessage::AuthFailure(AuthFailure { reason: "r".into() }).discriminator(),
    AUTH_FAILURE
  );
  assert_eq!(
    SignalingMessage::UserLogout(UserLogout::default()).discriminator(),
    USER_LOGOUT
  );
  assert_eq!(
    SignalingMessage::Ping(Ping::default()).discriminator(),
    PING
  );
  assert_eq!(
    SignalingMessage::Pong(Pong::default()).discriminator(),
    PONG
  );
  assert_eq!(
    SignalingMessage::SessionInvalidated(SessionInvalidated::default()).discriminator(),
    SESSION_INVALIDATED
  );
}

#[test]
fn test_discriminator_error_response() {
  use crate::error::{ErrorCategory, ErrorCode, ErrorModule};
  use std::collections::HashMap;
  let err = crate::ErrorResponse {
    code: ErrorCode::new(ErrorModule::Auth, ErrorCategory::Client, 1),
    message: "err".into(),
    i18n_key: "err".into(),
    details: HashMap::new(),
    timestamp_nanos: 0,
    trace_id: "trace".into(),
  };
  assert_eq!(
    SignalingMessage::ErrorResponse(err).discriminator(),
    ERROR_RESPONSE
  );
}
