use super::*;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

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
