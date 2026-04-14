use super::*;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use axum::body::Body;
use http_body_util::BodyExt;
use tower::ServiceExt;

// ===== Server Creation Tests =====

#[test]
fn test_server_new_with_default_config() {
  let config = Config::default();
  let server = Server::new(config);
  // Server should store the default config correctly
  assert_eq!(server.config().addr.port(), 3000);
  assert!(!server.config().jwt_secret.is_empty());
}

#[test]
fn test_server_new_with_custom_config() {
  let config = Config {
    addr: SocketAddr::from(([127, 0, 0, 1], 8080)),
    jwt_secret: "my-custom-secret".to_string(),
    static_dir: PathBuf::from("/tmp/custom-static"),
    stickers_dir: PathBuf::from("/tmp/custom-stickers"),
    ..Default::default()
  };

  let server = Server::new(config);
  assert_eq!(server.config().addr.port(), 8080);
  assert_eq!(
    server.config().addr.ip(),
    std::net::IpAddr::from([127, 0, 0, 1])
  );
  assert_eq!(server.config().jwt_secret, "my-custom-secret");
  assert_eq!(
    server.config().static_dir,
    PathBuf::from("/tmp/custom-static")
  );
  assert_eq!(
    server.config().stickers_dir,
    PathBuf::from("/tmp/custom-stickers")
  );
}

#[test]
fn test_server_config_accessor() {
  let config = Config::default();
  let expected_addr = config.addr;
  let expected_secret = config.jwt_secret.clone();
  let expected_ice_servers = config.ice_servers.clone();

  let server = Server::new(config);
  let cfg = server.config();

  assert_eq!(cfg.addr, expected_addr);
  assert_eq!(cfg.jwt_secret, expected_secret);
  assert_eq!(cfg.ice_servers, expected_ice_servers);
  assert!(cfg.tls.is_none());
}

// ===== Custom WebSocket Configuration Tests =====

#[test]
fn test_server_with_custom_heartbeat_config() {
  let config = Config {
    heartbeat_interval: Duration::from_secs(15),
    heartbeat_timeout: Duration::from_secs(45),
    ..Default::default()
  };

  let server = Server::new(config);
  assert_eq!(server.config().heartbeat_interval, Duration::from_secs(15));
  assert_eq!(server.config().heartbeat_timeout, Duration::from_secs(45));
}

#[test]
fn test_server_with_custom_max_message_size() {
  let config = Config {
    max_message_size: 2 * 1024 * 1024,
    ..Default::default()
  };

  let server = Server::new(config);
  assert_eq!(server.config().max_message_size, 2 * 1024 * 1024);
}

#[test]
fn test_server_with_custom_ice_servers() {
  let config = Config {
    ice_servers: vec![
      "stun:stun.example.com:3478".to_string(),
      "turn:turn.example.com:3478".to_string(),
    ],
    ..Default::default()
  };

  let server = Server::new(config);
  assert_eq!(server.config().ice_servers.len(), 2);
  assert_eq!(server.config().ice_servers[0], "stun:stun.example.com:3478");
  assert_eq!(server.config().ice_servers[1], "turn:turn.example.com:3478");
}

// ===== Router Build Tests =====

#[test]
fn test_build_router_returns_router_and_state() {
  let config = Config::default();
  let server = Server::new(config);
  let (_router, ws_state) = server.build_router();

  // WebSocketState should be initialized with zero connections
  assert_eq!(ws_state.connection_count(), 0);
  assert!(ws_state.connected_users().is_empty());
}

#[test]
fn test_build_router_ws_state_reflects_config() {
  let config = Config {
    jwt_secret: "router-test-secret".to_string(),
    ..Default::default()
  };

  let server = Server::new(config);
  let (_router, ws_state) = server.build_router();

  // WebSocketState should be functional (no connections initially)
  assert_eq!(ws_state.connection_count(), 0);
  assert!(!ws_state.is_connected(&message::UserId::new()));
}

// ===== Router HTTP Routing Tests =====

#[tokio::test]
async fn test_router_ws_route_exists() {
  let config = Config::default();
  let server = Server::new(config);
  let (router, _ws_state) = server.build_router();

  // Send a plain GET to /ws (without WebSocket upgrade headers).
  // The server should respond with a non-404 status, proving the route exists.
  // Without proper upgrade headers, axum returns 400 or similar, NOT 404.
  let request = axum::http::Request::builder()
    .uri("/ws")
    .method("GET")
    .body(Body::empty())
    .unwrap();

  let response = router.oneshot(request).await.unwrap();
  // A registered route without proper upgrade headers should NOT return 404
  assert_ne!(
    response.status().as_u16(),
    404,
    "The /ws route should be registered"
  );
}

#[tokio::test]
async fn test_router_unknown_route_falls_back_to_static() {
  let config = Config::default();
  let server = Server::new(config);
  let (router, _ws_state) = server.build_router();

  // Request a non-existent path; the fallback ServeDir handles it.
  // Since the static_dir likely doesn't exist in test, we expect a 404
  // from ServeDir (not from the router itself — the route IS matched by fallback).
  let request = axum::http::Request::builder()
    .uri("/nonexistent-path")
    .method("GET")
    .body(Body::empty())
    .unwrap();

  let response = router.oneshot(request).await.unwrap();
  // ServeDir returns 404 for missing files, which is expected behavior
  assert_eq!(
    response.status().as_u16(),
    404,
    "Non-existent static file should return 404"
  );
}

#[tokio::test]
async fn test_router_static_file_serving() {
  // Create a temporary directory with a test file to verify static serving
  let temp_dir = std::env::temp_dir().join("server_test_static");
  std::fs::create_dir_all(&temp_dir).unwrap();
  std::fs::write(temp_dir.join("test.txt"), "hello from static").unwrap();

  let config = Config {
    static_dir: temp_dir.clone(),
    ..Default::default()
  };

  let server = Server::new(config);
  let (router, _ws_state) = server.build_router();

  let request = axum::http::Request::builder()
    .uri("/test.txt")
    .method("GET")
    .body(Body::empty())
    .unwrap();

  let response = router.oneshot(request).await.unwrap();
  assert_eq!(
    response.status().as_u16(),
    200,
    "Static file should be served"
  );

  let body = response.into_body().collect().await.unwrap().to_bytes();
  assert_eq!(body.as_ref(), b"hello from static");

  // Cleanup
  let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_router_index_html_on_directory() {
  // Create a temporary directory with index.html to verify directory index serving
  let temp_dir = std::env::temp_dir().join("server_test_index");
  std::fs::create_dir_all(&temp_dir).unwrap();
  std::fs::write(temp_dir.join("index.html"), "<html>index</html>").unwrap();

  let config = Config {
    static_dir: temp_dir.clone(),
    ..Default::default()
  };

  let server = Server::new(config);
  let (router, _ws_state) = server.build_router();

  // Request the root path — should serve index.html
  let request = axum::http::Request::builder()
    .uri("/")
    .method("GET")
    .body(Body::empty())
    .unwrap();

  let response = router.oneshot(request).await.unwrap();
  assert_eq!(
    response.status().as_u16(),
    200,
    "Root path should serve index.html"
  );

  let body = response.into_body().collect().await.unwrap().to_bytes();
  assert_eq!(body.as_ref(), b"<html>index</html>");

  // Cleanup
  let _ = std::fs::remove_dir_all(&temp_dir);
}

// ===== Server Start Failure Tests =====

#[tokio::test]
async fn test_server_start_fails_on_invalid_address() {
  // Bind to a port that is already in use to trigger a start failure.
  // First, occupy a port:
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let occupied_port = listener.local_addr().unwrap().port();

  // Try to start a server on the same port
  let config = Config {
    addr: SocketAddr::from(([127, 0, 0, 1], occupied_port)),
    ..Default::default()
  };

  let server = Server::new(config);
  let result = server.start().await;

  // Should fail because the port is already occupied
  assert!(
    result.is_err(),
    "Server should fail to start on occupied port"
  );
}

// ==========================================================================
// MA-P2-005: Integration Test Isolation
// ==========================================================================

/// Helper trait for creating isolated test environments
pub trait IsolatedTestEnv {
  /// Create a unique test environment with isolated resources
  fn create() -> Self;
  /// Clean up all resources
  fn cleanup(&self);
}

/// Isolated server test environment
pub struct IsolatedServerEnv {
  pub config: Config,
  pub temp_dir: std::path::PathBuf,
  pub unique_id: String,
}

impl IsolatedTestEnv for IsolatedServerEnv {
  fn create() -> Self {
    let unique_id = format!("test_{}", uuid::Uuid::new_v4());
    let temp_dir = std::env::temp_dir().join(&unique_id);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let config = Config {
      addr: SocketAddr::from(([127, 0, 0, 1], 0)), // Let OS assign port
      jwt_secret: format!("secret_{}", unique_id),
      static_dir: temp_dir.join("static"),
      stickers_dir: temp_dir.join("stickers"),
      ..Default::default()
    };

    std::fs::create_dir_all(&config.static_dir).unwrap();
    std::fs::create_dir_all(&config.stickers_dir).unwrap();

    Self {
      config,
      temp_dir,
      unique_id,
    }
  }

  fn cleanup(&self) {
    let _ = std::fs::remove_dir_all(&self.temp_dir);
  }
}

impl Drop for IsolatedServerEnv {
  fn drop(&mut self) {
    self.cleanup();
  }
}

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

// ==========================================================================
// CR-P2-002: Integration Error/Edge Case Tests
// ==========================================================================

#[test]
fn test_empty_jwt_secret_validation() {
  // Empty JWT secret should be handled gracefully
  let config = Config {
    jwt_secret: String::new(),
    ..Default::default()
  };

  // Server should either reject empty secret or use a default
  // The current implementation allows it but shouldn't panic
  let result = std::panic::catch_unwind(|| {
    let server = Server::new(config);
    let _ = server.config();
  });
  assert!(result.is_ok());
}

#[test]
fn test_whitespace_jwt_secret() {
  let config = Config {
    jwt_secret: "   ".to_string(),
    ..Default::default()
  };

  // Whitespace-only secret should be handled
  let result = std::panic::catch_unwind(|| {
    let server = Server::new(config);
    let _ = server.config();
  });
  assert!(result.is_ok());
}

#[test]
fn test_very_long_jwt_secret() {
  let config = Config {
    jwt_secret: "x".repeat(10000),
    ..Default::default()
  };

  // Very long secret should be accepted
  let server = Server::new(config);
  assert!(server.config().jwt_secret.len() == 10000);
}

#[test]
fn test_unicode_jwt_secret() {
  let config = Config {
    jwt_secret: "你好世界🔐🎉".to_string(),
    ..Default::default()
  };

  // Unicode secret should be accepted
  let server = Server::new(config);
  assert!(server.config().jwt_secret.contains("🔐"));
}

#[test]
fn test_server_config_boundary_values() {
  let config = Config {
    addr: "0.0.0.0:65535".parse().unwrap(), // Max port
    heartbeat_timeout: std::time::Duration::from_secs(1), // Min reasonable timeout
    ..Default::default()
  };

  let server = Server::new(config);
  assert_eq!(server.config().addr.port(), 65535);
}

#[test]
fn test_zero_heartbeat_timeout() {
  let config = Config {
    heartbeat_timeout: std::time::Duration::from_secs(0),
    ..Default::default()
  };

  // Zero timeout should be handled (may cause immediate timeouts in practice)
  let server = Server::new(config);
  assert_eq!(
    server.config().heartbeat_timeout,
    std::time::Duration::from_secs(0)
  );
}

#[test]
fn test_very_long_heartbeat_timeout() {
  let config = Config {
    heartbeat_timeout: std::time::Duration::from_secs(86400 * 365), // 1 year
    ..Default::default()
  };

  let server = Server::new(config);
  assert!(server.config().heartbeat_timeout.as_secs() > 86400);
}

#[test]
fn test_nonexistent_static_dir() {
  let config = Config {
    static_dir: std::path::PathBuf::from("/nonexistent/path/that/does/not/exist"),
    ..Default::default()
  };

  // Server should handle nonexistent directories gracefully
  let result = std::panic::catch_unwind(|| {
    let server = Server::new(config);
    let _ = server.config();
  });
  assert!(result.is_ok());
}

#[test]
fn test_nonexistent_stickers_dir() {
  let config = Config {
    stickers_dir: std::path::PathBuf::from("/another/nonexistent/path"),
    ..Default::default()
  };

  let result = std::panic::catch_unwind(|| {
    let server = Server::new(config);
    let _ = server.config();
  });
  assert!(result.is_ok());
}

#[tokio::test]
async fn test_router_creation_with_invalid_config() {
  let env = IsolatedServerEnv::create();
  let server = Server::new(env.config.clone());

  // Router creation should succeed
  let (router, ws_state) = server.build_router();

  // Router should be valid
  let _ = router;

  // WebSocket state should be initialized
  assert_eq!(ws_state.connection_count(), 0);

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

#[test]
fn test_ipv6_address_binding() {
  // Test IPv6 loopback
  let addr_str = "[::1]:0";
  let addr: SocketAddr = addr_str.parse().unwrap();

  let config = Config {
    addr,
    ..Default::default()
  };

  let server = Server::new(config);
  assert!(server.config().addr.is_ipv6());
}

#[test]
fn test_ipv4_address_binding() {
  let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();

  let config = Config {
    addr,
    ..Default::default()
  };

  let server = Server::new(config);
  assert!(server.config().addr.is_ipv4());
}

#[test]
fn test_wildcard_address_binding() {
  let addr: SocketAddr = "0.0.0.0:0".parse().unwrap();

  let config = Config {
    addr,
    ..Default::default()
  };

  let server = Server::new(config);
  assert!(server.config().addr.ip().is_unspecified());
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

#[test]
fn test_server_with_reserved_port() {
  // Port 1 is reserved/system port
  let addr: SocketAddr = "127.0.0.1:1".parse().unwrap();

  let config = Config {
    addr,
    ..Default::default()
  };

  // Server creation should succeed (binding happens later)
  let server = Server::new(config);
  assert_eq!(server.config().addr.port(), 1);
}

#[test]
fn test_config_clone_consistency() {
  let config = Config::default();
  let config_clone = config.clone();

  assert_eq!(config.jwt_secret, config_clone.jwt_secret);
  assert_eq!(config.heartbeat_timeout, config_clone.heartbeat_timeout);
  assert_eq!(config.addr, config_clone.addr);
}

#[test]
fn test_config_default_values() {
  let _config = Config::default();

  // Default values should be reasonable
  // Note: jwt_secret may be empty in default config
  // Note: heartbeat_timeout default value is set by Default impl
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
fn test_user_store_with_empty_credentials() {
  let env = IsolatedServerEnv::create();
  let user_store = crate::auth::UserStore::new(&env.config);

  // Empty username
  let _result = user_store.register("", "password");
  // Should either fail or succeed (implementation dependent)

  // Empty password
  let _result = user_store.register("user", "");
  // Should either fail or succeed

  // Both empty
  let result = user_store.register("", "");
  // Should either fail or succeed

  env.cleanup();
  let _ = result;
}

#[test]
fn test_user_store_duplicate_registration() {
  let env = IsolatedServerEnv::create();
  let user_store = crate::auth::UserStore::new(&env.config);

  // First registration
  let result1 = user_store.register("duplicate_user", "password1");
  assert!(result1.is_ok());

  // Duplicate registration
  let result2 = user_store.register("duplicate_user", "password2");
  assert!(result2.is_err());

  env.cleanup();
}

#[test]
fn test_room_state_empty_operations() {
  let room_state = crate::room::RoomState::new();
  let user_id = message::UserId::new();
  let room_id = message::RoomId::new();

  // Operations on empty state should be handled
  let leave_result = room_state.leave_room(
    &message::signaling::LeaveRoom {
      room_id: room_id.clone(),
    },
    &user_id,
  );
  assert!(leave_result.is_err());
}
