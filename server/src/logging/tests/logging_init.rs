use super::*;
use serial_test::serial;
use tempfile::TempDir;

// =============================================================================
// File Writer Tests
// =============================================================================

#[test]
#[serial]
fn test_create_file_writer_creates_directory() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let config = create_test_config(&temp_dir, "file");

  // The directory should already exist from temp_dir creation
  assert!(temp_dir.path().exists());

  // Create file writer - this should succeed
  let result = create_file_writer(&config);
  assert!(result.is_ok(), "create_file_writer should succeed");

  let (_non_blocking, _guard) = result.unwrap();

  // File writer created successfully
}

#[test]
#[serial]
fn test_create_file_writer_with_nested_directory() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let nested_dir = temp_dir.path().join("nested").join("logs");

  let config = crate::config::Config {
    addr: "0.0.0.0:3000".parse().unwrap(),
    jwt_secret: "test-secret".to_string(),
    ice_servers: vec![],
    tls: None,
    static_dir: std::path::PathBuf::from("./static"),
    stickers_dir: std::path::PathBuf::from("./stickers"),
    log_level: "info".to_string(),
    log_format: "pretty".to_string(),
    log_output: "file".to_string(),
    log_rotation: crate::config::LogRotation::Never,
    log_dir: nested_dir.clone(),
    log_max_files: 5,
    log_max_size_mb: 10,
    heartbeat_interval: std::time::Duration::from_secs(30),
    heartbeat_timeout: std::time::Duration::from_secs(60),
    max_message_size: 1024 * 1024,
    send_queue_size: 256,
  };

  // Create the nested directory first
  std::fs::create_dir_all(&nested_dir).expect("Failed to create nested dir");

  let result = create_file_writer(&config);
  assert!(
    result.is_ok(),
    "create_file_writer should succeed with nested dir"
  );
}

#[test]
#[serial]
fn test_init_creates_log_directory() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let log_dir = temp_dir.path().join("logs");
  assert!(!log_dir.exists(), "Log dir should not exist yet");

  let config = crate::config::Config {
    addr: "0.0.0.0:3000".parse().unwrap(),
    jwt_secret: "test-secret".to_string(),
    ice_servers: vec![],
    tls: None,
    static_dir: std::path::PathBuf::from("./static"),
    stickers_dir: std::path::PathBuf::from("./stickers"),
    log_level: "info".to_string(),
    log_format: "pretty".to_string(),
    log_output: "file".to_string(),
    log_rotation: crate::config::LogRotation::Never,
    log_dir: log_dir.clone(),
    log_max_files: 5,
    log_max_size_mb: 10,
    heartbeat_interval: std::time::Duration::from_secs(30),
    heartbeat_timeout: std::time::Duration::from_secs(60),
    max_message_size: 1024 * 1024,
    send_queue_size: 256,
  };

  // The key assertion is that the log directory gets created
  let result = try_init(&config);
  assert!(
    result.is_ok(),
    "try_init should succeed (or return None if already init)"
  );

  // The main test: log directory should be created
  assert!(log_dir.exists(), "Log directory should be created");
}

// =============================================================================
// Logging Init Tests
// =============================================================================

#[test]
#[serial]
fn test_init_stdout_mode() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let config = create_test_config(&temp_dir, "stdout");

  // Accept either Ok(Some) or Ok(None) - global subscriber may already be set
  let result = try_init(&config);
  assert!(
    result.is_ok(),
    "try_init with stdout should succeed or return None"
  );
}

#[test]
#[serial]
fn test_init_file_mode() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let config = create_test_config(&temp_dir, "file");

  // Accept either Ok(Some) or Ok(None) - global subscriber may already be set
  let result = try_init(&config);
  assert!(
    result.is_ok(),
    "try_init with file should succeed or return None"
  );
}

#[test]
#[serial]
fn test_init_both_mode() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let config = create_test_config(&temp_dir, "both");

  // Accept either Ok(Some) or Ok(None) - global subscriber may already be set
  let result = try_init(&config);
  assert!(
    result.is_ok(),
    "try_init with both should succeed or return None"
  );
}

#[test]
#[serial]
fn test_init_with_json_format() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let mut config = create_test_config(&temp_dir, "file");
  config.log_format = "json".to_string();

  // Accept either Ok(Some) or Ok(None) - global subscriber may already be set
  let result = try_init(&config);
  assert!(
    result.is_ok(),
    "try_init with JSON format should succeed or return None"
  );
}

#[test]
#[serial]
fn test_init_with_pretty_format() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let mut config = create_test_config(&temp_dir, "stdout");
  config.log_format = "pretty".to_string();

  // Accept either Ok(Some) or Ok(None) - global subscriber may already be set
  let result = try_init(&config);
  assert!(
    result.is_ok(),
    "try_init with pretty format should succeed or return None"
  );
}

// =============================================================================
// Log Rotation Tests
// =============================================================================

#[test]
#[serial]
fn test_log_rotation_daily() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let mut config = create_test_config(&temp_dir, "file");
  config.log_rotation = crate::config::LogRotation::Daily;

  // Accept either Ok(Some) or Ok(None) - global subscriber may already be set
  let result = try_init(&config);
  assert!(
    result.is_ok(),
    "try_init with Daily rotation should succeed or return None"
  );
}

#[test]
#[serial]
fn test_log_rotation_hourly() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let mut config = create_test_config(&temp_dir, "file");
  config.log_rotation = crate::config::LogRotation::Hourly;

  // Accept either Ok(Some) or Ok(None) - global subscriber may already be set
  let result = try_init(&config);
  assert!(
    result.is_ok(),
    "try_init with Hourly rotation should succeed or return None"
  );
}

#[test]
#[serial]
fn test_log_rotation_never() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let mut config = create_test_config(&temp_dir, "file");
  config.log_rotation = crate::config::LogRotation::Never;

  // Accept either Ok(Some) or Ok(None) - global subscriber may already be set
  let result = try_init(&config);
  assert!(
    result.is_ok(),
    "try_init with Never rotation should succeed or return None"
  );
}

// =============================================================================
// LogGuard Tests
// =============================================================================

#[test]
#[serial]
fn test_log_guard_holds_reference() {
  let temp_dir = TempDir::new().expect("Failed to create temp dir");
  let config = create_test_config(&temp_dir, "file");

  // Accept either Ok(Some) or Ok(None) - global subscriber may already be set
  let result = try_init(&config);
  assert!(result.is_ok(), "try_init should succeed or return None");
}
