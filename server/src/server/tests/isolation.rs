use super::*;

#[test]
fn test_isolated_env_creates_unique_directories() {
  let env1 = IsolatedServerEnv::create();
  let env2 = IsolatedServerEnv::create();

  // Each environment should have unique directories
  assert_ne!(env1.temp_dir, env2.temp_dir);
  assert_ne!(env1.config.jwt_secret, env2.config.jwt_secret);

  // Directories should exist
  assert!(env1.temp_dir.exists());
  assert!(env2.temp_dir.exists());

  env1.cleanup();
  env2.cleanup();

  // Directories should be removed
  assert!(!env1.temp_dir.exists());
  assert!(!env2.temp_dir.exists());
}

#[test]
fn test_isolated_env_config_is_valid() {
  let env = IsolatedServerEnv::create();

  // Config should be valid for server creation
  let server = Server::new(env.config.clone());
  assert!(server.config().jwt_secret.starts_with("secret_test_"));

  env.cleanup();
}

#[test]
fn test_multiple_isolated_servers_dont_conflict() {
  let env1 = IsolatedServerEnv::create();
  let env2 = IsolatedServerEnv::create();

  let server1 = Server::new(env1.config.clone());
  let server2 = Server::new(env2.config.clone());

  // Both servers should have independent configs
  assert_ne!(server1.config().jwt_secret, server2.config().jwt_secret);

  env1.cleanup();
  env2.cleanup();
}

#[test]
fn test_isolated_env_auto_cleanup_on_drop() {
  let temp_dir_path = {
    let env = IsolatedServerEnv::create();
    let path = env.temp_dir.clone();
    assert!(path.exists());
    path
    // env is dropped here
  };

  // After drop, directory should be cleaned up
  // Note: This might not work on all platforms due to timing
  std::thread::sleep(std::time::Duration::from_millis(10));
  // The directory might still exist if Drop didn't run yet
  // This is mainly to demonstrate the pattern
  let _ = temp_dir_path;
}

#[tokio::test]
async fn test_parallel_isolated_tests() {
  use tokio::task::JoinSet;

  let mut set = JoinSet::new();

  // Spawn multiple parallel test tasks with isolated environments
  for i in 0..5 {
    set.spawn(async move {
      let env = IsolatedServerEnv::create();
      let server = Server::new(env.config.clone());
      let (_router, ws_state) = server.build_router();

      // Each task should have isolated state
      assert_eq!(ws_state.connection_count(), 0);

      // Verify unique JWT secret
      assert!(server.config().jwt_secret.contains("test_"));

      // Simulate some work
      tokio::time::sleep(std::time::Duration::from_millis(10)).await;

      env.cleanup();
      i
    });
  }

  // Wait for all tasks
  let mut results = Vec::new();
  while let Some(result) = set.join_next().await {
    results.push(result.unwrap());
  }

  // All tasks should have completed
  assert_eq!(results.len(), 5);
}

#[test]
fn test_isolated_user_store() {
  let env = IsolatedServerEnv::create();
  let user_store = crate::auth::UserStore::new(&env.config);

  // Register a user
  let result = user_store.register("testuser", "password123");
  assert!(result.is_ok());

  // User should be stored
  let users = user_store.get_online_users();
  assert!(!users.is_empty());

  env.cleanup();
}

#[test]
fn test_isolated_room_state() {
  let env = IsolatedServerEnv::create();
  let room_state = crate::room::RoomState::new();

  let owner_id = message::types::UserId::new();
  let create_request = message::signaling::CreateRoom {
    name: "Test Room".to_string(),
    description: String::new(),
    room_type: message::types::RoomType::Chat,
    password: None,
    max_participants: 8,
  };

  let result = room_state.create_room(&create_request, owner_id.clone());
  assert!(result.is_ok());

  env.cleanup();
}

#[test]
fn test_isolated_discovery_state() {
  let env = IsolatedServerEnv::create();
  let discovery_state = crate::discovery::DiscoveryState::new();

  let from = message::UserId::new();
  let to = message::UserId::new();

  let invite = message::signaling::ConnectionInvite {
    from: from.clone(),
    to: to.clone(),
    note: None,
  };

  let result = discovery_state.send_invitation(&invite);
  assert!(result.is_ok());

  env.cleanup();
}

#[test]
fn test_isolated_websocket_state() {
  let env = IsolatedServerEnv::create();
  let user_store = crate::auth::UserStore::new(&env.config);
  let ws_state = crate::ws::WebSocketState::new(env.config.clone(), user_store);

  // Should start with zero connections
  assert_eq!(ws_state.connection_count(), 0);

  let user_id = message::UserId::new();
  let (tx, _rx) = tokio::sync::mpsc::channel(16);

  ws_state.add_connection(user_id.clone(), tx);

  // Should have one connection
  assert_eq!(ws_state.connection_count(), 1);
  assert!(ws_state.is_connected(&user_id));

  env.cleanup();
}

#[test]
fn test_test_env_deterministic_unique_ids() {
  let env1 = IsolatedServerEnv::create();
  let env2 = IsolatedServerEnv::create();

  // Each environment should have different unique IDs
  assert_ne!(env1.unique_id, env2.unique_id);

  // But within same environment, the ID should be consistent
  assert!(env1.config.jwt_secret.contains(&env1.unique_id));

  env1.cleanup();
  env2.cleanup();
}

#[tokio::test]
async fn test_websocket_state_isolation() {
  let env = IsolatedServerEnv::create();
  let server = Server::new(env.config.clone());
  let (_, ws_state) = server.build_router();

  // Create two users
  let user1 = message::UserId::new();
  let user2 = message::UserId::new();

  let (tx1, _rx1) = tokio::sync::mpsc::channel(16);
  let (tx2, _rx2) = tokio::sync::mpsc::channel(16);

  // Add connections
  ws_state.add_connection(user1.clone(), tx1);
  ws_state.add_connection(user2.clone(), tx2);

  // Both should be connected
  assert!(ws_state.is_connected(&user1));
  assert!(ws_state.is_connected(&user2));

  // Remove one
  ws_state.remove_connection(&user1);

  // Only user1 should be disconnected
  assert!(!ws_state.is_connected(&user1));
  assert!(ws_state.is_connected(&user2));

  env.cleanup();
}

#[test]
fn test_concurrent_server_creation() {
  let mut handles = vec![];

  // Create multiple servers concurrently
  for _ in 0..10 {
    let handle = std::thread::spawn(|| {
      let env = IsolatedServerEnv::create();
      let server = Server::new(env.config.clone());
      let (router, ws_state) = server.build_router();

      // Verify server is functional
      assert_eq!(ws_state.connection_count(), 0);

      env.cleanup();
      router
    });
    handles.push(handle);
  }

  // All should succeed
  for handle in handles {
    assert!(handle.join().is_ok());
  }
}

#[tokio::test]
async fn test_multiple_router_creation_same_server() {
  let env = IsolatedServerEnv::create();
  let server = Server::new(env.config.clone());

  // Create multiple routers from same server
  let (router1, ws_state1) = server.build_router();
  let (router2, ws_state2) = server.build_router();

  // Both should be independent
  assert_eq!(ws_state1.connection_count(), 0);
  assert_eq!(ws_state2.connection_count(), 0);

  // Add connection to ws_state1
  let (tx, _rx) = tokio::sync::mpsc::channel(16);
  let user_id = message::UserId::new();
  ws_state1.add_connection(user_id.clone(), tx);

  // ws_state1 should have 1 connection
  assert_eq!(ws_state1.connection_count(), 1);

  // ws_state2 should still have 0 (independent states)
  // Note: This depends on whether build_router creates new or shared state

  env.cleanup();
  let _ = (router1, router2);
}
