//! Application configuration.
//!
//! Runtime configuration loaded from Trunk environment variables
//! and browser environment detection. Provided once via Leptos context
//! to avoid re-detecting the environment on every access.

use leptos::prelude::*;

/// Application configuration.
#[derive(Debug, Clone)]
pub struct Config {
  /// WebSocket URL for signaling server.
  /// Falls back to constructing from `window.location`.
  pub ws_url: String,
  /// HTTP base URL for API requests (register, login, etc.).
  pub http_url: String,
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
    let http_url = Self::detect_http_url();
    let debug = Self::detect_debug_mode();
    let version = env!("CARGO_PKG_VERSION").to_string();

    Self {
      ws_url,
      http_url,
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

          // In development mode, the Trunk dev server (port 8080) proxies
          // HTTP API requests correctly but has known issues with WebSocket
          // binary frame proxying (Trunk bug). When running on port 8080,
          // connect directly to the backend server on port 3000 instead.
          let ws_host = if host.ends_with(":8080") {
            host.replace(":8080", ":3000")
          } else {
            host
          };

          format!("{}://{}/ws", ws_protocol, ws_host)
        } else {
          "ws://localhost:3000/ws".to_string()
        }
      },
      |url| url.to_string(),
    )
  }

  /// Detect HTTP base URL for API requests.
  ///
  /// Uses `TRUNK_HTTP_URL` environment variable if set,
  /// otherwise constructs from current browser location.
  fn detect_http_url() -> String {
    option_env!("TRUNK_HTTP_URL").map_or_else(
      || {
        if let Some(window) = web_sys::window() {
          let location = window.location();
          let protocol = location.protocol().unwrap_or_default();
          let host = location.host().unwrap_or_default();
          // protocol() returns e.g. "https:" — strip trailing colon to
          // avoid producing "https:://host".
          let scheme = protocol.trim_end_matches(':');
          format!("{}://{}", scheme, host)
        } else {
          "http://localhost:8080".to_string()
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

// ── Context helpers ──

/// Provide Config to the Leptos component tree.
///
/// Creates the config once and stores it via context so all consumers
/// share the same instance.
pub fn provide_config() -> Config {
  let config = Config::new();
  provide_context(config.clone());
  config
}

/// Retrieve Config from Leptos context.
///
/// # Panics
/// Panics if `provide_config` has not been called.
#[must_use]
pub fn use_config() -> Config {
  expect_context::<Config>()
}

#[cfg(test)]
mod tests;
