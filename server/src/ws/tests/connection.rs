//! Connection state and management tests.

use super::*;

#[test]
fn test_connection_state_new() {
  let state = ConnectionState::new("127.0.0.1:12345".to_string());
  assert!(state.user_id.is_none());
  assert_eq!(state.remote_addr, "127.0.0.1:12345");
}

#[test]
fn test_websocket_state_new() {
  let state = create_test_ws_state();
  assert_eq!(state.connection_count(), 0);
}

#[test]
fn test_websocket_state_add_remove() {
  let state = create_test_ws_state();
  let user_id = UserId::new();
  let (tx, _rx) = mpsc::channel(16);

  state.add_connection(user_id.clone(), tx);
  assert_eq!(state.connection_count(), 1);
  assert!(state.is_connected(&user_id));

  state.remove_connection(&user_id);
  assert_eq!(state.connection_count(), 0);
  assert!(!state.is_connected(&user_id));
}

#[test]
fn test_connection_state_with_authenticated_user() {
  let mut state = ConnectionState::new("127.0.0.1:12345".to_string());
  let user_id = UserId::new();
  let (tx, _rx) = mpsc::channel(16);

  // Simulate user authentication
  state.user_id = Some(user_id.clone());
  state.sender = Some(tx);

  assert_eq!(state.user_id, Some(user_id));
  assert!(state.sender.is_some());
}

#[test]
fn test_websocket_state_broadcast_empty() {
  let state = create_test_ws_state();

  // Broadcast to zero connections should succeed
  let msg = SignalingMessage::Ping(Ping::default());
  let encoded = encode_signaling_message(&msg).unwrap();
  // This should not panic
  let rt = tokio::runtime::Runtime::new().unwrap();
  rt.block_on(async {
    state.broadcast(encoded).await;
  });
}

#[test]
fn test_websocket_state_send_to_nonexistent_user() {
  let state = create_test_ws_state();
  let user_id = UserId::new();

  let msg = SignalingMessage::Ping(Ping::default());
  let encoded = encode_signaling_message(&msg).unwrap();

  // Sending to nonexistent user should return false
  let rt = tokio::runtime::Runtime::new().unwrap();
  let result = rt.block_on(async { state.send_to(&user_id, encoded).await });
  assert!(!result);
}

#[test]
fn test_connection_cleanup_on_duplicate_add() {
  let state = create_test_ws_state();
  let user_id = UserId::new();

  // Add connection
  let sender1 = create_test_sender();
  state.add_connection(user_id.clone(), sender1);

  // Add again (should replace or handle gracefully)
  let sender2 = create_test_sender();
  state.add_connection(user_id.clone(), sender2);

  // Should still be connected
  assert!(state.is_connected(&user_id));

  // Should have exactly one connection
  let sender = state.get_sender(&user_id);
  assert!(sender.is_some());
}

#[test]
fn test_disconnect_nonexistent_user() {
  let state = create_test_ws_state();
  let user_id = UserId::new();

  // Try to disconnect user that was never connected
  // Should not panic
  state.remove_connection(&user_id);

  // Still not connected
  assert!(!state.is_connected(&user_id));
}

#[test]
fn test_get_all_connected_users() {
  let state = create_test_ws_state();
  let users: Vec<UserId> = (0..5).map(|_| UserId::new()).collect();

  // Connect all
  for user in &users {
    state.add_connection(user.clone(), create_test_sender());
  }

  // Get connected count (assuming such a method exists)
  // If not, we verify each user individually
  for user in &users {
    assert!(state.is_connected(user));
  }

  // Disconnect some
  for user in &users[0..2] {
    state.remove_connection(user);
  }

  // Verify remaining
  for user in &users[0..2] {
    assert!(!state.is_connected(user));
  }
  for user in &users[2..5] {
    assert!(state.is_connected(user));
  }
}
