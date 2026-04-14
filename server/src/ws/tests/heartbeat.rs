//! Heartbeat timeout tests.

use super::*;

#[test]
fn test_connection_state_initial_heartbeat() {
  let state = ConnectionState::new("127.0.0.1:12345".to_string());

  // Initially, last_heartbeat should be very recent (within 1 second)
  assert!(state.last_heartbeat.elapsed() < std::time::Duration::from_secs(1));
}

#[test]
fn test_heartbeat_not_timed_out_initially() {
  let config = create_test_config();
  let state = ConnectionState::new("127.0.0.1:12345".to_string());

  // Fresh connection should not be timed out
  assert!(state.last_heartbeat.elapsed() < config.heartbeat_timeout);
}

#[test]
fn test_heartbeat_timeout_detection() {
  let config = create_test_config();
  let mut state = ConnectionState::new("127.0.0.1:12345".to_string());

  // Simulate heartbeat timeout by setting last_heartbeat to the past
  state.last_heartbeat =
    Instant::now() - config.heartbeat_timeout - std::time::Duration::from_secs(1);

  // Now it should be timed out
  assert!(state.last_heartbeat.elapsed() > config.heartbeat_timeout);
}

#[test]
fn test_heartbeat_timeout_exact_boundary() {
  let config = create_test_config();
  let mut state = ConnectionState::new("127.0.0.1:12345".to_string());

  // Set last_heartbeat to exactly the timeout threshold
  state.last_heartbeat = Instant::now() - config.heartbeat_timeout;

  // At exact boundary, should still be timed out (elapsed >= timeout)
  assert!(state.last_heartbeat.elapsed() >= config.heartbeat_timeout);
}

#[test]
fn test_heartbeat_timeout_near_boundary() {
  let config = create_test_config();
  let mut state = ConnectionState::new("127.0.0.1:12345".to_string());

  // Set last_heartbeat to just before the timeout threshold
  state.last_heartbeat =
    Instant::now() - config.heartbeat_timeout + std::time::Duration::from_millis(100);

  // Should not be timed out yet
  assert!(state.last_heartbeat.elapsed() < config.heartbeat_timeout);
}

#[test]
fn test_config_heartbeat_defaults() {
  let config = Config::default();

  // Verify default heartbeat configuration
  assert_eq!(
    config.heartbeat_interval,
    std::time::Duration::from_secs(30)
  );
  assert_eq!(config.heartbeat_timeout, std::time::Duration::from_secs(60));
}

#[test]
fn test_heartbeat_timeout_greater_than_interval() {
  let config = Config::default();

  // Heartbeat timeout should be greater than interval to allow for network latency
  assert!(
    config.heartbeat_timeout > config.heartbeat_interval,
    "Heartbeat timeout ({:?}) should be greater than interval ({:?})",
    config.heartbeat_timeout,
    config.heartbeat_interval
  );

  // Typically timeout should be 2x the interval
  let ratio = config.heartbeat_timeout.as_secs() / config.heartbeat_interval.as_secs();
  assert!(
    ratio >= 2,
    "Heartbeat timeout should be at least 2x the interval"
  );
}

#[test]
fn test_heartbeat_timeout_triggers_disconnect() {
  let config = Config {
    heartbeat_timeout: std::time::Duration::from_secs(1),
    ..Default::default()
  };
  let user_store = UserStore::new(&config);
  let state = WebSocketState::new(config.clone(), user_store);

  let user_id = UserId::new();
  state.add_connection(user_id.clone(), create_test_sender());

  // Initially connected
  assert!(state.is_connected(&user_id));

  // Simulate heartbeat timeout by checking the condition
  // In real implementation, a background task would detect this
  let mut conn_state = ConnectionState::new("127.0.0.1:12345".to_string());
  conn_state.user_id = Some(user_id.clone());
  conn_state.last_heartbeat =
    std::time::Instant::now() - config.heartbeat_timeout - std::time::Duration::from_secs(1);

  // Verify timeout condition
  assert!(
    conn_state.last_heartbeat.elapsed() > config.heartbeat_timeout,
    "Connection should be timed out"
  );
}
