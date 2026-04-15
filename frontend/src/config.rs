//! Application configuration.
//!
//! Runtime configuration loaded from Trunk environment variables
//! and browser environment detection.

/// Application configuration.
#[derive(Debug, Clone)]
pub struct Config {
  /// WebSocket URL for signaling server.
  /// Falls back to constructing from `window.location`.
  pub ws_url: String,
  /// Debug mode enabled.
  pub debug: bool,
  /// Application version (embedded at build time).
  pub version: String,
}

impl Config {
  /// Create configuration by detecting browser environment.
  #[must_use]
  pub fn new() -> Self {
    let ws_url = Self::detect_ws_url();
    let debug = Self::detect_debug_mode();
    let version = env!("CARGO_PKG_VERSION").to_string();

    Self {
      ws_url,
      debug,
      version,
    }
  }

  /// Detect WebSocket URL.
  ///
  /// Uses `TRUNK_WS_URL` environment variable if set,
  /// otherwise constructs from current browser location.
  fn detect_ws_url() -> String {
    // Check for build-time override via Trunk
    option_env!("TRUNK_WS_URL").map_or_else(
      || {
        // Runtime fallback: construct from window.location
        if let Some(window) = web_sys::window() {
          let location = window.location();
          let protocol = location.protocol().unwrap_or_default();
          let host = location.host().unwrap_or_default();
          let ws_protocol = if protocol == "https:" { "wss" } else { "ws" };
          format!("{}://{}", ws_protocol, host)
        } else {
          "ws://localhost:8080".to_string()
        }
      },
      |url| url.to_string(),
    )
  }

  /// Detect debug mode.
  ///
  /// Checks `TRUNK_DEBUG` environment variable or URL query parameter.
  fn detect_debug_mode() -> bool {
    option_env!("TRUNK_DEBUG")
      .map(|v| v == "true")
      .unwrap_or_else(|| {
        // Runtime check: look for ?debug=true in URL
        if let Some(window) = web_sys::window() {
          let location = window.location();
          if let Ok(search) = location.search() {
            return search.contains("debug=true");
          }
        }
        false
      })
  }
}

impl Default for Config {
  fn default() -> Self {
    Self::new()
  }
}
