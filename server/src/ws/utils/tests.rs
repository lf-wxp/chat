use super::*;
use message::frame::decode_frame;
use message::signaling::{Ping, TokenAuth};

#[test]
fn test_encode_signaling_message() {
  let msg = SignalingMessage::TokenAuth(TokenAuth {
    token: "test_token".to_string(),
  });
  let result = encode_signaling_message(&msg);
  assert!(result.is_ok());

  let encoded = result.unwrap();
  // Should start with magic number 0xBCBC
  assert_eq!(encoded[0], 0xBC);
  assert_eq!(encoded[1], 0xBC);
  // Next byte is discriminator
  assert_eq!(encoded[2], 0x00); // TOKEN_AUTH discriminator
}

#[test]
fn test_decode_signaling_message() {
  let msg = SignalingMessage::Ping(Ping::default());
  let encoded = encode_signaling_message(&msg).unwrap();
  let frame = decode_frame(&encoded).unwrap();
  let decoded = decode_signaling_message(&frame);

  assert!(decoded.is_ok());
  assert!(matches!(decoded.unwrap(), SignalingMessage::Ping(_)));
}

// =============================================================================
// parse_error_code Tests
// =============================================================================

#[test]
fn test_parse_error_code_sig() {
  let code = parse_error_code("SIG001");
  assert_eq!(code.module, ErrorModule::Sig);
  assert_eq!(code.sequence, 1);
}

#[test]
fn test_parse_error_code_rom() {
  let code = parse_error_code("ROM701");
  assert_eq!(code.module, ErrorModule::Rom);
  assert_eq!(code.sequence, 701);
}

#[test]
fn test_parse_error_code_auth() {
  let code = parse_error_code("AUTH100");
  assert_eq!(code.module, ErrorModule::Auth);
  assert_eq!(code.sequence, 100);
}

#[test]
fn test_parse_error_code_cht() {
  let code = parse_error_code("CHT050");
  assert_eq!(code.module, ErrorModule::Cht);
  assert_eq!(code.sequence, 50);
}

#[test]
fn test_parse_error_code_e2e() {
  let code = parse_error_code("E2E200");
  assert_eq!(code.module, ErrorModule::E2e);
  assert_eq!(code.sequence, 200);
}

#[test]
fn test_parse_error_code_fil() {
  let code = parse_error_code("FIL300");
  assert_eq!(code.module, ErrorModule::Fil);
  assert_eq!(code.sequence, 300);
}

#[test]
fn test_parse_error_code_thr() {
  let code = parse_error_code("THR400");
  assert_eq!(code.module, ErrorModule::Thr);
  assert_eq!(code.sequence, 400);
}

#[test]
fn test_parse_error_code_pst() {
  let code = parse_error_code("PST500");
  assert_eq!(code.module, ErrorModule::Pst);
  assert_eq!(code.sequence, 500);
}

#[test]
fn test_parse_error_code_sys() {
  let code = parse_error_code("SYS999");
  assert_eq!(code.module, ErrorModule::Sys);
  assert_eq!(code.sequence, 999);
}

#[test]
fn test_parse_error_code_unknown_prefix() {
  let code = parse_error_code("XXX123");
  // Unknown prefix defaults to Sys module
  assert_eq!(code.module, ErrorModule::Sys);
}

#[test]
fn test_parse_error_code_short_string() {
  let code = parse_error_code("AB");
  // Too short for 3-char prefix
  assert_eq!(code.module, ErrorModule::Sys);
}

#[test]
fn test_parse_error_code_empty_string() {
  let code = parse_error_code("");
  assert_eq!(code.module, ErrorModule::Sys);
}

#[test]
fn test_parse_error_code_invalid_numeric() {
  let code = parse_error_code("SIGABC");
  assert_eq!(code.module, ErrorModule::Sig);
  // Non-numeric should default to 0
  assert_eq!(code.sequence, 0);
}

#[test]
fn test_parse_error_code_no_numeric() {
  let code = parse_error_code("SIG");
  assert_eq!(code.module, ErrorModule::Sig);
  assert_eq!(code.sequence, 0);
}

// =============================================================================
// encode/decode roundtrip Tests
// =============================================================================

#[test]
fn test_encode_decode_roundtrip_auth_success() {
  let msg = SignalingMessage::AuthSuccess(message::signaling::AuthSuccess {
    user_id: message::UserId::new(),
    username: "testuser".to_string(),
    nickname: "testuser".to_string(),
  });
  let encoded = encode_signaling_message(&msg).unwrap();
  let frame = decode_frame(&encoded).unwrap();
  let decoded = decode_signaling_message(&frame);

  assert!(decoded.is_ok());
  match decoded.unwrap() {
    SignalingMessage::AuthSuccess(auth) => {
      assert_eq!(auth.username, "testuser");
    }
    _ => panic!("Expected AuthSuccess"),
  }
}

#[test]
fn test_encode_decode_roundtrip_error_response() {
  let msg = SignalingMessage::ErrorResponse(message::ErrorResponse {
    code: message::error::ErrorCode::new(ErrorModule::Sig, ErrorCategory::Client, 1),
    message: "Test error".to_string(),
    i18n_key: "error.test".to_string(),
    details: std::collections::HashMap::new(),
    timestamp_nanos: 0,
    trace_id: "trace-123".to_string(),
  });
  let encoded = encode_signaling_message(&msg).unwrap();
  let frame = decode_frame(&encoded).unwrap();
  let decoded = decode_signaling_message(&frame);

  assert!(decoded.is_ok());
  match decoded.unwrap() {
    SignalingMessage::ErrorResponse(err) => {
      assert_eq!(err.message, "Test error");
    }
    _ => panic!("Expected ErrorResponse"),
  }
}

#[test]
fn test_encode_decode_roundtrip_user_status_change() {
  let msg = SignalingMessage::UserStatusChange(message::signaling::UserStatusChange {
    user_id: message::UserId::new(),
    status: message::types::UserStatus::Away,
    signature: Some("Hello world".to_string()),
  });
  let encoded = encode_signaling_message(&msg).unwrap();
  let frame = decode_frame(&encoded).unwrap();
  let decoded = decode_signaling_message(&frame);

  assert!(decoded.is_ok());
  match decoded.unwrap() {
    SignalingMessage::UserStatusChange(status) => {
      assert_eq!(status.status, message::types::UserStatus::Away);
      assert_eq!(status.signature, Some("Hello world".to_string()));
    }
    _ => panic!("Expected UserStatusChange"),
  }
}

#[test]
fn test_encode_decode_roundtrip_pong() {
  let msg = SignalingMessage::Pong(message::signaling::Pong::default());
  let encoded = encode_signaling_message(&msg).unwrap();
  let frame = decode_frame(&encoded).unwrap();
  let decoded = decode_signaling_message(&frame);

  assert!(decoded.is_ok());
  assert!(matches!(decoded.unwrap(), SignalingMessage::Pong(_)));
}
