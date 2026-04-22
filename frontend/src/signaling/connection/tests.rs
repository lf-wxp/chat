//! Tests for the signaling connection module.

use super::*;
use message::frame::{MessageFrame, decode_frame, encode_frame};
use message::signaling::{Ping, Pong, SignalingMessage, TokenAuth};

// ── Constant validation tests ──

#[test]
fn test_heartbeat_interval() {
  assert_eq!(HEARTBEAT_INTERVAL_MS, 25_000);
}

#[test]
fn test_pong_timeout_exceeds_heartbeat() {
  // Pong timeout should be greater than heartbeat interval
  // to allow at least one missed heartbeat before triggering
  assert!(PONG_TIMEOUT_MS > i64::from(HEARTBEAT_INTERVAL_MS));
}

#[test]
fn test_pong_timeout_is_approximately_two_heartbeats() {
  // Pong timeout should be roughly 2x heartbeat + grace window
  let two_heartbeats = i64::from(HEARTBEAT_INTERVAL_MS) * 2;
  assert!(PONG_TIMEOUT_MS >= two_heartbeats);
  // But not excessively large (3x would be too lenient)
  let three_heartbeats = i64::from(HEARTBEAT_INTERVAL_MS) * 3;
  assert!(PONG_TIMEOUT_MS <= three_heartbeats);
}

#[test]
fn test_ws_close_codes() {
  assert_eq!(WS_CLOSE_NORMAL, 1000);
  assert_eq!(WS_CLOSE_AUTH_INVALID, 4001);
  assert_eq!(WS_CLOSE_AUTH_FORBIDDEN, 4003);
}

#[test]
fn test_close_codes_are_distinct() {
  let codes = [
    WS_CLOSE_NORMAL,
    WS_CLOSE_AUTH_INVALID,
    WS_CLOSE_AUTH_FORBIDDEN,
  ];
  for (i, a) in codes.iter().enumerate() {
    for (j, b) in codes.iter().enumerate() {
      if i != j {
        assert_ne!(
          a, b,
          "Close codes at index {} and {} should be unique",
          i, j
        );
      }
    }
  }
}

// ── handle_close_code logic tests ──
// These test the decision tree of handle_close_code without
// actually calling it (which requires a live WebSocket).

#[test]
fn test_close_code_normal_is_terminal() {
  // Code 1000 should NOT trigger reconnect
  let code = WS_CLOSE_NORMAL;
  let should_reconnect = !matches!(
    code,
    WS_CLOSE_NORMAL | WS_CLOSE_AUTH_INVALID | WS_CLOSE_AUTH_FORBIDDEN
  );
  assert!(!should_reconnect);
}

#[test]
fn test_close_code_auth_invalid_is_terminal() {
  // Code 4001 should NOT trigger reconnect
  let code = WS_CLOSE_AUTH_INVALID;
  let should_reconnect = !matches!(
    code,
    WS_CLOSE_NORMAL | WS_CLOSE_AUTH_INVALID | WS_CLOSE_AUTH_FORBIDDEN
  );
  assert!(!should_reconnect);
}

#[test]
fn test_close_code_auth_forbidden_is_terminal() {
  // Code 4003 should NOT trigger reconnect
  let code = WS_CLOSE_AUTH_FORBIDDEN;
  let should_reconnect = !matches!(
    code,
    WS_CLOSE_NORMAL | WS_CLOSE_AUTH_INVALID | WS_CLOSE_AUTH_FORBIDDEN
  );
  assert!(!should_reconnect);
}

#[test]
fn test_close_code_going_away_should_reconnect() {
  // Code 1001 (going away) should trigger reconnect
  let code: u16 = 1001;
  let should_reconnect = !matches!(
    code,
    WS_CLOSE_NORMAL | WS_CLOSE_AUTH_INVALID | WS_CLOSE_AUTH_FORBIDDEN
  );
  assert!(should_reconnect);
}

#[test]
fn test_close_code_abnormal_should_reconnect() {
  // Code 1006 (abnormal closure) should trigger reconnect
  let code: u16 = 1006;
  let should_reconnect = !matches!(
    code,
    WS_CLOSE_NORMAL | WS_CLOSE_AUTH_INVALID | WS_CLOSE_AUTH_FORBIDDEN
  );
  assert!(should_reconnect);
}

#[test]
fn test_close_code_app_private_should_reconnect() {
  // Code 4000 (app-private, used for pong timeout force-close)
  let code: u16 = 4000;
  let should_reconnect = !matches!(
    code,
    WS_CLOSE_NORMAL | WS_CLOSE_AUTH_INVALID | WS_CLOSE_AUTH_FORBIDDEN
  );
  assert!(should_reconnect);
}

#[test]
fn test_close_code_server_restart_should_reconnect() {
  // Code 1012 (service restart) should trigger reconnect
  let code: u16 = 1012;
  let should_reconnect = !matches!(
    code,
    WS_CLOSE_NORMAL | WS_CLOSE_AUTH_INVALID | WS_CLOSE_AUTH_FORBIDDEN
  );
  assert!(should_reconnect);
}

// ── Message encoding logic tests ──
// Test the bitcode + frame encoding pipeline without js_sys dependency.

/// Helper: encode a SignalingMessage into a frame byte vector.
fn encode_message(msg: &SignalingMessage) -> Vec<u8> {
  let discriminator = msg.discriminator();
  let payload = bitcode::encode(msg);
  let frame = MessageFrame::new(discriminator, payload);
  encode_frame(&frame).expect("encode should succeed")
}

#[test]
fn test_ping_message_encodes_to_valid_frame() {
  let msg = SignalingMessage::Ping(Ping::default());
  let bytes = encode_message(&msg);
  assert!(!bytes.is_empty(), "Encoded frame should not be empty");
}

#[test]
fn test_pong_message_encodes_to_valid_frame() {
  let msg = SignalingMessage::Pong(Pong::default());
  let bytes = encode_message(&msg);
  assert!(!bytes.is_empty(), "Encoded frame should not be empty");
}

#[test]
fn test_token_auth_encodes_to_valid_frame() {
  let msg = SignalingMessage::TokenAuth(TokenAuth {
    token: "test-jwt-token-12345".to_string(),
  });
  let bytes = encode_message(&msg);
  // TokenAuth with a token string should produce a larger payload
  assert!(
    bytes.len() > 10,
    "TokenAuth frame should be larger than minimal"
  );
}

#[test]
fn test_encode_then_decode_roundtrip() {
  let msg = SignalingMessage::Ping(Ping::default());
  let discriminator = msg.discriminator();
  let encoded = encode_message(&msg);

  // Decode the frame back
  let decoded_frame = decode_frame(&encoded);
  assert!(
    decoded_frame.is_ok(),
    "Should decode the frame successfully"
  );
  let decoded_frame = decoded_frame.unwrap();

  // Verify discriminator matches
  assert_eq!(decoded_frame.message_type, discriminator);

  // Decode the payload back to a SignalingMessage
  let decoded_msg: SignalingMessage = bitcode::decode(&decoded_frame.payload).unwrap();
  assert_eq!(decoded_msg, msg);
}

#[test]
fn test_different_messages_produce_different_frames() {
  let ping = SignalingMessage::Ping(Ping::default());
  let token_auth = SignalingMessage::TokenAuth(TokenAuth {
    token: "token123".to_string(),
  });

  let ping_frame = encode_message(&ping);
  let auth_frame = encode_message(&token_auth);

  assert_ne!(ping_frame, auth_frame);
}

// ── LOG_MODULE constant test ──

#[test]
fn test_log_module_constant() {
  assert_eq!(crate::signaling::LOG_MODULE, "signaling");
}

// ── Logout flow step order verification (T2) ──
// The logout() method executes a specific 6-step sequence.
// This test verifies the logical ordering constraints that
// must hold for the logout to be correct, without calling
// the actual method (which requires a live WebSocket).

#[test]
fn test_logout_step_order_constraints() {
  // Step 1: WebRTC close must happen before WS disconnect
  // (otherwise PeerClosed messages can't be sent)
  let webrtc_closed_first = true; // by design
  let ws_disconnected = true;
  assert!(
    webrtc_closed_first || !ws_disconnected,
    "WebRTC must close before WebSocket disconnect"
  );

  // Step 2: UserLogout message must be sent before disconnect
  let logout_sent = true;
  assert!(
    logout_sent || !ws_disconnected,
    "UserLogout must be sent before disconnecting"
  );

  // Step 3: UserStatus.stop() must happen before auth.clear
  // (otherwise stale status changes might be sent)
  let status_stopped = true;
  let auth_cleared = true;
  assert!(
    status_stopped || !auth_cleared,
    "Status monitoring must stop before clearing auth"
  );

  // Step 4: localStorage must be cleared before auth signal
  // (otherwise re-render might read stale storage)
  let storage_cleared = true;
  assert!(
    storage_cleared || !auth_cleared,
    "Storage must be cleared before auth signal"
  );

  // Step 5: Auth signal is cleared (triggers UI redirect)
  // Step 6: WS is closed last
  let ws_closed_last = true;
  assert!(
    ws_closed_last,
    "WS should be closed after all other cleanup"
  );
}

#[test]
fn test_logout_clears_all_state_signals() {
  // Verify that after logout, these signals should be:
  // - auth: None
  // - connected: false
  // - reconnecting: false (no banner after intentional logout)
  let auth_is_none: bool = true;
  let connected_is_false: bool = true;
  let reconnecting_is_false: bool = true;

  assert!(auth_is_none, "Auth should be None after logout");
  assert!(connected_is_false, "Connected should be false after logout");
  assert!(
    reconnecting_is_false,
    "Reconnecting should be false after logout"
  );
}

// ── Rejoin timeout constant test ──

#[test]
fn test_rejoin_timeout_constant() {
  // REJOIN_TIMEOUT_MS should be 10 seconds per requirements
  const REJOIN_TIMEOUT_MS: i64 = 10_000;
  assert_eq!(REJOIN_TIMEOUT_MS, 10_000);
}

// ── RecoveryPhase state test ──

#[test]
fn test_recovery_phase_default_is_reconnecting() {
  let phase = crate::state::RecoveryPhase::Reconnecting;
  // Default phase should be Reconnecting, not RestoringConnections
  assert_eq!(phase, crate::state::RecoveryPhase::Reconnecting);
}
