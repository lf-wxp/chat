use super::*;
use tempfile::TempDir;

/// Helper to create a test config with a temp directory.
pub(super) fn create_test_config(temp_dir: &TempDir, log_output: &str) -> crate::config::Config {
  crate::config::Config {
    addr: "0.0.0.0:3000".parse().unwrap(),
    jwt_secret: "test-secret".to_string(),
    ice_servers: vec![],
    stun_port: None,
    tls: None,
    static_dir: std::path::PathBuf::from("./static"),
    stickers_dir: std::path::PathBuf::from("./stickers"),
    log_level: "info".to_string(),
    log_format: "pretty".to_string(),
    log_output: log_output.to_string(),
    log_rotation: crate::config::LogRotation::Never,
    log_dir: temp_dir.path().to_path_buf(),
    log_max_files: 5,
    log_max_size_mb: 10,
    heartbeat_interval: std::time::Duration::from_secs(30),
    heartbeat_timeout: std::time::Duration::from_secs(60),
    max_message_size: 1024 * 1024,
    send_queue_size: 256,
  }
}

mod desensitize;
mod log_cleanup;
mod logging_init;
