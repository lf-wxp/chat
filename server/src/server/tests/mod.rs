use super::*;
use std::net::SocketAddr;

/// Helper trait for creating isolated test environments
pub(super) trait IsolatedTestEnv {
  /// Create a unique test environment with isolated resources
  fn create() -> Self;
  /// Clean up all resources
  fn cleanup(&self);
}

/// Isolated server test environment
pub(super) struct IsolatedServerEnv {
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

mod edge_cases;
mod isolation;
mod router_http;
mod server_creation;
