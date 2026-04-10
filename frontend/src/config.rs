//! Frontend configuration.

/// Frontend configuration.
#[derive(Debug, Clone)]
pub struct Config {
  /// WebSocket server URL
  pub ws_url: String,
  /// API base URL
  pub api_url: String,
  /// Debug mode enabled
  pub debug: bool,
  /// Default locale
  pub locale: String,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      ws_url: "ws://localhost:3000/ws".to_string(),
      api_url: "http://localhost:3000".to_string(),
      debug: false,
      locale: "en".to_string(),
    }
  }
}

impl Config {
  /// Create configuration from browser environment.
  #[must_use]
  pub fn from_browser() -> Self {
    let mut config = Self::default();

    // Check for debug mode in URL
    if let Some(window) = web_sys::window() {
      if let Ok(href) = window.location().href() {
        config.debug = href.contains("debug=true");
      }

      // Check localStorage for debug mode
      if let Ok(Some(storage)) = window.local_storage() {
        if let Ok(Some(debug)) = storage.get_item("debug_mode") {
          config.debug = debug == "true";
        }
        if let Ok(Some(locale)) = storage.get_item("locale") {
          config.locale = locale;
        }
      }
    }

    config
  }
}
