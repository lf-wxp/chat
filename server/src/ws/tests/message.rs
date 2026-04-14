//! Message encoding and decoding tests.

use super::*;

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

#[test]
fn test_session_invalidated_message() {
  let msg = SignalingMessage::SessionInvalidated(SessionInvalidated::default());
  let result = encode_signaling_message(&msg);
  assert!(result.is_ok());

  let encoded = result.unwrap();
  assert_eq!(encoded[0], 0xBC);
  assert_eq!(encoded[1], 0xBC);
  assert_eq!(encoded[2], 0x07); // SESSION_INVALIDATED discriminator

  let frame = decode_frame(&encoded).unwrap();
  let decoded = decode_signaling_message(&frame);
  assert!(decoded.is_ok());
  assert!(matches!(
    decoded.unwrap(),
    SignalingMessage::SessionInvalidated(_)
  ));
}

#[test]
fn test_ping_message_encoding() {
  // Test that Ping message can be encoded correctly
  let ping_msg = SignalingMessage::Ping(Ping::default());
  let result = encode_signaling_message(&ping_msg);
  assert!(result.is_ok());

  let encoded = result.unwrap();
  assert_eq!(encoded[0], 0xBC);
  assert_eq!(encoded[1], 0xBC);
  assert_eq!(encoded[2], 0x04); // PING discriminator
}

#[test]
fn test_pong_message_encoding() {
  // Test that Pong message can be encoded correctly
  let pong_msg = SignalingMessage::Pong(Pong::default());
  let result = encode_signaling_message(&pong_msg);
  assert!(result.is_ok());

  let encoded = result.unwrap();
  assert_eq!(encoded[0], 0xBC);
  assert_eq!(encoded[1], 0xBC);
  assert_eq!(encoded[2], 0x05); // PONG discriminator
}
