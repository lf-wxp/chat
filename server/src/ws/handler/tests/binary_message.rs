//! Tests for binary message handling.
//!
//! Tests:
//! - Valid Ping binary message
//! - Invalid frame data
//! - Invalid signaling data

use super::*;

// -----------------------------------------------------------------------------
// Binary Message Handling
// -----------------------------------------------------------------------------

#[tokio::test]
async fn test_handle_binary_valid_ping() {
  let mut ctx = TestContext::new();

  // Create a valid Ping message
  let ping_msg = SignalingMessage::Ping(message::signaling::Ping);
  let encoded = TestContext::create_encoded_message(&ping_msg);

  // Handle the binary message
  let result = handle_binary_message(
    &mut ctx.mock_sink,
    &ctx.ws_state,
    &mut ctx.conn_state,
    encoded,
  )
  .await;

  assert!(result, "Valid Ping should not close connection");

  // Verify a Pong response was sent
  if let Some(response) = ctx.receive_message().await {
    assert!(
      matches!(response, Message::Binary(_)),
      "Response should be Binary message"
    );
  } else {
    panic!("Expected a response message");
  }
}

#[tokio::test]
async fn test_handle_binary_invalid_frame() {
  let mut ctx = TestContext::new();

  // Create invalid frame data
  let invalid_data = TestContext::create_invalid_frame_data();

  // Handle the invalid binary message
  let result = handle_binary_message(
    &mut ctx.mock_sink,
    &ctx.ws_state,
    &mut ctx.conn_state,
    invalid_data,
  )
  .await;

  // Invalid frame should return true (continue connection)
  assert!(result, "Invalid frame should not close connection");
}

#[tokio::test]
async fn test_handle_binary_invalid_signaling() {
  let mut ctx = TestContext::new();

  // Create a valid frame but with invalid signaling data
  // Frame with valid magic but invalid message type
  let mut data = vec![0xBC, 0xBC]; // Magic number
  data.push(0xFF); // Invalid message type discriminator
  data.push(0x00); // Reserved

  // Add some payload
  data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

  // Handle the binary message
  let result =
    handle_binary_message(&mut ctx.mock_sink, &ctx.ws_state, &mut ctx.conn_state, data).await;

  // Invalid signaling should return true (continue connection)
  assert!(
    result,
    "Invalid signaling message should not close connection"
  );
}
