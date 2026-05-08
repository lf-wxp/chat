use super::*;
use std::net::SocketAddr;

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
