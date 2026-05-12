//! Tests for token authentication handling.
//!
//! Tests:
//! - Successful token authentication
//! - Failed token authentication
//! - Re-authentication rejection

use super::*;

// -----------------------------------------------------------------------------
// Token Authentication Tests
// -----------------------------------------------------------------------------

#[tokio::test]
async fn test_token_auth_success() {
  let mut ctx = TestContext::new();

  // Create a valid token
  let token = create_valid_token(&ctx.ws_state, "testuser");

  // Create TokenAuth message
  let auth_msg = SignalingMessage::TokenAuth(TokenAuth { token });
  let encoded = TestContext::create_encoded_message(&auth_msg);

  // Handle the binary message
  let result = handle_binary_message(
    &mut ctx.mock_sink,
    &ctx.ws_state,
    &mut ctx.conn_state,
    encoded,
  )
  .await;

  assert!(result, "Successful auth should not close connection");

  // Verify user is now authenticated
  assert!(
    ctx.conn_state.user_id.is_some(),
    "User should be authenticated"
  );

  // Verify AuthSuccess response was sent
  if let Some(response) = ctx.receive_message().await {
    assert!(
      matches!(response, Message::Binary(_)),
      "Response should be Binary message"
    );
  }
}

#[tokio::test]
async fn test_token_auth_failure() {
  let mut ctx = TestContext::new();

  // Create an invalid token
  let token = create_invalid_token();

  // Create TokenAuth message with invalid token
  let auth_msg = SignalingMessage::TokenAuth(TokenAuth { token });
  let encoded = TestContext::create_encoded_message(&auth_msg);

  // Handle the binary message
  let result = handle_binary_message(
    &mut ctx.mock_sink,
    &ctx.ws_state,
    &mut ctx.conn_state,
    encoded,
  )
  .await;

  // Auth failure should return true (allow retry)
  assert!(result, "Auth failure should not close connection");

  // Verify user is not authenticated
  assert!(
    ctx.conn_state.user_id.is_none(),
    "User should not be authenticated"
  );

  // Verify AuthFailure response was sent
  if let Some(response) = ctx.receive_message().await {
    assert!(
      matches!(response, Message::Binary(_)),
      "Response should be Binary message"
    );
  }
}

#[tokio::test]
async fn test_reauth_rejected() {
  let mut ctx = TestContext::with_authenticated_user("testuser");

  // Try to authenticate again
  let token = create_valid_token(&ctx.ws_state, "testuser2");
  let auth_msg = SignalingMessage::TokenAuth(TokenAuth { token });
  let encoded = TestContext::create_encoded_message(&auth_msg);

  // Handle the binary message
  let result = handle_binary_message(
    &mut ctx.mock_sink,
    &ctx.ws_state,
    &mut ctx.conn_state,
    encoded,
  )
  .await;

  // Re-auth should return false (close connection)
  assert!(!result, "Re-authentication should close connection");
}

// -----------------------------------------------------------------------------
// Unauthenticated User Tests
// -----------------------------------------------------------------------------

#[tokio::test]
async fn test_unauthenticated_user_sends_message() {
  let mut ctx = TestContext::new();

  // Send a message that requires authentication (e.g., ConnectionInvite)
  let invite_msg = SignalingMessage::ConnectionInvite(ConnectionInvite {
    from: message::UserId::new(),
    to: message::UserId::new(),
    note: None,
  });
  let encoded = TestContext::create_encoded_message(&invite_msg);

  // Handle the binary message
  let result = handle_binary_message(
    &mut ctx.mock_sink,
    &ctx.ws_state,
    &mut ctx.conn_state,
    encoded,
  )
  .await;

  // Unauthenticated user should be disconnected
  assert!(
    !result,
    "Unauthenticated user sending message should close connection"
  );

  // Verify AuthFailure response was sent
  if let Some(response) = ctx.receive_message().await {
    assert!(
      matches!(response, Message::Binary(_)),
      "Response should be Binary message"
    );
  }
}
