//! Tests for handle_user_disconnect.
//!
//! Tests:
//! - User disconnect removes connection and updates status
//! - User disconnect cleans up sender

use super::*;
use crate::ws::handler::handle_user_disconnect;

// -----------------------------------------------------------------------------
// handle_user_disconnect Tests
// -----------------------------------------------------------------------------

#[tokio::test]
async fn test_handle_user_disconnect() {
  let ctx = TestContext::with_authenticated_user("testuser");
  let user_id = ctx.conn_state.user_id.clone().unwrap();

  // Verify user is connected
  assert!(ctx.ws_state.is_connected(&user_id));
  assert_eq!(
    ctx.ws_state.user_store.get_user(&user_id).unwrap().status,
    UserStatus::Online
  );

  // Handle disconnect
  handle_user_disconnect(&ctx.ws_state, &user_id).await;

  // Verify user is disconnected
  assert!(!ctx.ws_state.is_connected(&user_id));
  assert_eq!(
    ctx.ws_state.user_store.get_user(&user_id).unwrap().status,
    UserStatus::Offline
  );
}

#[tokio::test]
async fn test_handle_user_disconnect_cleans_up_sender() {
  let ctx = TestContext::with_authenticated_user("testuser");
  let user_id = ctx.conn_state.user_id.clone().unwrap();

  // Verify sender exists
  assert!(ctx.ws_state.get_sender(&user_id).is_some());

  // Handle disconnect
  handle_user_disconnect(&ctx.ws_state, &user_id).await;

  // Verify sender is removed
  assert!(ctx.ws_state.get_sender(&user_id).is_none());
}
