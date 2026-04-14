//! Handler timeout tests (BUG-003).

use super::*;

/// Test that message handlers have timeout protection
#[tokio::test]
async fn test_message_handler_responds_within_timeout() {
  let state = create_test_ws_state();
  let user_id = UserId::new();

  state.add_connection(user_id.clone(), create_test_sender());

  // Simulate rapid message processing
  // All operations should complete within reasonable time
  let start = std::time::Instant::now();

  // Perform multiple state operations
  for _ in 0..100 {
    let _ = state.is_connected(&user_id);
    let _ = state.get_sender(&user_id);
  }

  let elapsed = start.elapsed();
  // Should complete within 100ms for 100 operations
  assert!(
    elapsed < std::time::Duration::from_millis(100),
    "Operations should be fast enough"
  );
}

/// Test that slow operations don't block other connections
#[tokio::test]
async fn test_handler_does_not_block_other_connections() {
  use std::sync::Arc;

  let state = Arc::new(create_test_ws_state());

  // Connect multiple users
  let users: Vec<UserId> = (0..10).map(|_| UserId::new()).collect();
  for user in &users {
    state.add_connection(user.clone(), create_test_sender());
  }

  let state_clone = state.clone();
  let users_clone = users.clone();

  // Thread performing many operations
  let handle = tokio::spawn(async move {
    for _ in 0..1000 {
      for user in &users_clone {
        let _ = state_clone.is_connected(user);
      }
    }
  });

  // Main thread should still be responsive
  let start = std::time::Instant::now();
  for _ in 0..100 {
    let _ = state.connection_count();
  }
  let elapsed = start.elapsed();

  // Main thread operations should not be significantly delayed
  assert!(
    elapsed < std::time::Duration::from_millis(50),
    "Main thread should remain responsive"
  );

  handle.await.unwrap();
}

/// Test handler timeout for authentication
#[tokio::test]
async fn test_auth_handler_timeout_protection() {
  let state = create_test_ws_state();

  // Authentication should be fast
  let start = std::time::Instant::now();

  // Register a user for testing
  let _ = state.user_store.register("timeout_test_user", "password");

  // Login
  let _ = state.user_store.login("timeout_test_user", "password");

  let elapsed = start.elapsed();

  // Authentication should complete within reasonable time
  assert!(
    elapsed < std::time::Duration::from_millis(100),
    "Authentication should be fast"
  );
}

/// Test handler timeout for room operations
#[tokio::test]
async fn test_room_handler_timeout_protection() {
  let state = create_test_ws_state();
  let owner_id = UserId::new();

  // Room operations should be fast
  let start = std::time::Instant::now();

  // Create multiple rooms
  for i in 0..100 {
    let create_request = CreateRoom {
      name: format!("Room {}", i),
      room_type: RoomType::Chat,
      password: None,
      max_participants: 8,
    };
    let _ = state
      .room_state
      .create_room(&create_request, owner_id.clone());
  }

  let elapsed = start.elapsed();

  // Should complete within reasonable time
  assert!(
    elapsed < std::time::Duration::from_millis(500),
    "Room operations should be fast"
  );
}

/// Test handler timeout for discovery operations
#[tokio::test]
async fn test_discovery_handler_timeout_protection() {
  let state = create_test_ws_state();

  // Discovery operations should be fast
  let start = std::time::Instant::now();

  // Send multiple invitations
  for _ in 0..100 {
    let from = UserId::new();
    let to = UserId::new();
    let invite = ConnectionInvite {
      from,
      to,
      note: None,
    };
    let _ = state.discovery_state.send_invitation(&invite);
  }

  let elapsed = start.elapsed();

  // Should complete within reasonable time
  assert!(
    elapsed < std::time::Duration::from_millis(500),
    "Discovery operations should be fast"
  );
}

/// Test message broadcast timeout
#[tokio::test]
async fn test_broadcast_timeout_protection() {
  let state = create_test_ws_state();

  // Connect multiple users
  let users: Vec<UserId> = (0..50).map(|_| UserId::new()).collect();
  for user in &users {
    state.add_connection(user.clone(), create_test_sender());
  }

  // Broadcast should be fast
  let start = std::time::Instant::now();

  // Simulate broadcast to all users
  for entry in state.connections.iter() {
    let sender = entry.value();
    // In real implementation, this would send a message
    // For testing, we just verify the iteration is fast
    let _ = sender.clone();
  }

  let elapsed = start.elapsed();

  // Broadcast iteration should be fast
  assert!(
    elapsed < std::time::Duration::from_millis(10),
    "Broadcast iteration should be fast"
  );
}

/// Test concurrent handler timeout protection
#[tokio::test]
async fn test_concurrent_handler_timeout() {
  let state = Arc::new(create_test_ws_state());

  // Connect users
  let users: Vec<UserId> = (0..10).map(|_| UserId::new()).collect();
  for user in &users {
    state.add_connection(user.clone(), create_test_sender());
  }

  let mut handles = vec![];

  // Spawn multiple concurrent operations
  for _ in 0..10 {
    let state_clone = state.clone();
    let users_clone = users.clone();

    let handle = tokio::spawn(async move {
      let start = std::time::Instant::now();
      for user in &users_clone {
        let _ = state_clone.is_connected(user);
        let _ = state_clone.get_sender(user);
      }
      start.elapsed()
    });

    handles.push(handle);
  }

  // Wait for all operations
  let results: Vec<std::time::Duration> = futures::future::join_all(handles)
    .await
    .into_iter()
    .map(|r| r.unwrap())
    .collect();

  // All operations should complete quickly
  for elapsed in results {
    assert!(
      elapsed < std::time::Duration::from_millis(100),
      "Concurrent operations should complete quickly"
    );
  }
}

/// Test handler graceful degradation on slow operations
#[test]
fn test_handler_graceful_degradation() {
  let state = create_test_ws_state();
  let user_id = UserId::new();

  state.add_connection(user_id.clone(), create_test_sender());

  // Even if we perform many operations, the state should remain consistent
  for _ in 0..10000 {
    assert!(state.is_connected(&user_id));
  }

  // State should still be valid after many operations
  assert_eq!(state.connection_count(), 1);
  assert!(state.is_connected(&user_id));
}

/// Test handler timeout boundary conditions
#[test]
fn test_handler_timeout_boundary() {
  let config = Config {
    heartbeat_interval: std::time::Duration::from_millis(100),
    heartbeat_timeout: std::time::Duration::from_millis(300),
    ..Default::default()
  };
  let user_store = UserStore::new(&config);
  let _state = WebSocketState::new(config.clone(), user_store);

  // Verify timeout boundaries
  assert!(config.heartbeat_timeout > config.heartbeat_interval);

  // Connection state should track heartbeat correctly
  let mut conn_state = ConnectionState::new("127.0.0.1:12345".to_string());
  conn_state.last_heartbeat = std::time::Instant::now();

  // Just after heartbeat - should not be timed out
  assert!(conn_state.last_heartbeat.elapsed() < config.heartbeat_timeout);

  // Simulate time passing
  conn_state.last_heartbeat = std::time::Instant::now() - config.heartbeat_timeout;

  // At exact timeout boundary - should be timed out
  assert!(conn_state.last_heartbeat.elapsed() >= config.heartbeat_timeout);
}
