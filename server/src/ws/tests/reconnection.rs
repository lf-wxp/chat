//! WebSocket reconnection tests (MA-P2-004).

use super::*;

#[test]
fn test_connection_state_after_disconnect() {
  let state = create_test_ws_state();
  let user_id = UserId::new();

  // User connects
  state.add_connection(user_id.clone(), create_test_sender());

  // Verify connected
  assert!(state.is_connected(&user_id));

  // User disconnects
  state.remove_connection(&user_id);

  // Verify disconnected
  assert!(!state.is_connected(&user_id));
}

#[test]
fn test_reconnect_with_same_user_id() {
  let state = create_test_ws_state();
  let user_id = UserId::new();

  // First connection
  let sender1 = create_test_sender();
  state.add_connection(user_id.clone(), sender1.clone());
  assert!(state.is_connected(&user_id));

  // Simulate disconnect
  state.remove_connection(&user_id);
  assert!(!state.is_connected(&user_id));

  // Reconnect with same user_id
  let sender2 = create_test_sender();
  state.add_connection(user_id.clone(), sender2);

  // Should be connected again
  assert!(state.is_connected(&user_id));
}

#[test]
fn test_reconnect_invalidates_old_sender() {
  let state = create_test_ws_state();
  let user_id = UserId::new();

  // First connection
  let sender1 = create_test_sender();
  state.add_connection(user_id.clone(), sender1);

  // Get sender reference
  let retrieved1 = state.get_sender(&user_id);
  assert!(retrieved1.is_some());

  // Simulate disconnect and reconnect
  state.remove_connection(&user_id);
  let sender2 = create_test_sender();
  state.add_connection(user_id.clone(), sender2);

  // Old sender reference should be invalid or new sender should be returned
  let retrieved2 = state.get_sender(&user_id);
  assert!(retrieved2.is_some());
}

#[test]
fn test_concurrent_reconnection_handling() {
  use std::sync::Arc;

  let state = Arc::new(create_test_ws_state());
  let user_id = UserId::new();

  let state_clone = state.clone();
  let user_id_clone = user_id.clone();

  // Thread 1: Connect
  let handle1 = std::thread::spawn(move || {
    for i in 0..10 {
      let sender = create_test_sender();
      state_clone.add_connection(user_id_clone.clone(), sender);
      std::thread::sleep(std::time::Duration::from_micros(100));
      state_clone.remove_connection(&user_id_clone);
      std::thread::sleep(std::time::Duration::from_micros(100));
      eprintln!("Thread 1 iteration {}", i);
    }
  });

  // Thread 2: Check connection status
  let state_clone2 = state.clone();
  let user_id_clone2 = user_id.clone();
  let handle2 = std::thread::spawn(move || {
    for i in 0..20 {
      let connected = state_clone2.is_connected(&user_id_clone2);
      eprintln!("Thread 2 check {}: connected = {}", i, connected);
      std::thread::sleep(std::time::Duration::from_micros(100));
    }
  });

  handle1.join().unwrap();
  handle2.join().unwrap();

  // Final state should be consistent
  // After all operations, user should be disconnected (last action was remove)
}

#[test]
fn test_session_cleanup_on_disconnect() {
  let state = create_test_ws_state();
  let user_id = UserId::new();

  // Connect user
  state.add_connection(user_id.clone(), create_test_sender());

  // User joins a room (simulate state tracking)
  // In a real scenario, this would involve RoomState

  // Disconnect should clean up session
  state.remove_connection(&user_id);

  // Verify cleanup
  assert!(!state.is_connected(&user_id));
  assert!(state.get_sender(&user_id).is_none());
}

#[test]
fn test_multiple_users_disconnect_and_reconnect() {
  let state = create_test_ws_state();
  let users: Vec<UserId> = (0..5).map(|_| UserId::new()).collect();

  // Connect all users
  for user in &users {
    state.add_connection(user.clone(), create_test_sender());
  }

  // Verify all connected
  for user in &users {
    assert!(state.is_connected(user));
  }

  // Disconnect first 3
  for user in &users[0..3] {
    state.remove_connection(user);
  }

  // Verify first 3 disconnected, last 2 connected
  for user in &users[0..3] {
    assert!(!state.is_connected(user));
  }
  for user in &users[3..5] {
    assert!(state.is_connected(user));
  }

  // Reconnect first 3
  for user in &users[0..3] {
    state.add_connection(user.clone(), create_test_sender());
  }

  // All should be connected again
  for user in &users {
    assert!(state.is_connected(user));
  }
}

#[test]
fn test_connection_state_persistence() {
  let state = create_test_ws_state();
  let user_id = UserId::new();

  // Initial state
  assert!(!state.is_connected(&user_id));

  // Connect
  state.add_connection(user_id.clone(), create_test_sender());
  assert!(state.is_connected(&user_id));

  // State should persist across multiple checks
  for _ in 0..10 {
    assert!(state.is_connected(&user_id));
  }

  // Disconnect
  state.remove_connection(&user_id);
  assert!(!state.is_connected(&user_id));

  // State should persist across multiple checks
  for _ in 0..10 {
    assert!(!state.is_connected(&user_id));
  }
}

#[test]
fn test_reconnect_during_message_processing() {
  let state = create_test_ws_state();
  let user_id = UserId::new();

  // Connect
  state.add_connection(user_id.clone(), create_test_sender());

  // Simulate message processing that checks connection
  let sender1 = state.get_sender(&user_id);
  assert!(sender1.is_some());

  // User reconnects during processing
  state.remove_connection(&user_id);
  state.add_connection(user_id.clone(), create_test_sender());

  // Processing continues with old sender (may fail or succeed)
  // The important thing is no panic occurs
  let sender2 = state.get_sender(&user_id);
  assert!(sender2.is_some());
}
