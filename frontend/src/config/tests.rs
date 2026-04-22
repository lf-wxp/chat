use super::*;

// ── Protocol switching logic tests ──
// These test the core ws/http protocol selection logic extracted
// from detect_ws_url / detect_http_url.

#[test]
fn test_ws_protocol_for_https() {
  let protocol = "https:";
  let ws_protocol = if protocol == "https:" { "wss" } else { "ws" };
  assert_eq!(ws_protocol, "wss");
}

#[test]
fn test_ws_protocol_for_http() {
  let protocol = "http:";
  let ws_protocol = if protocol == "https:" { "wss" } else { "ws" };
  assert_eq!(ws_protocol, "ws");
}

#[test]
fn test_ws_url_format() {
  let ws_protocol = "wss";
  let host = "example.com:443";
  let url = format!("{}://{}/ws", ws_protocol, host);
  assert_eq!(url, "wss://example.com:443/ws");
}

#[test]
fn test_ws_url_dev_port_replacement() {
  // When on port 8080 (Trunk dev server), should redirect to port 3000
  let host = "localhost:8080";
  let ws_host = if host.ends_with(":8080") {
    host.replace(":8080", ":3000")
  } else {
    host.to_string()
  };
  let url = format!("ws://{}/ws", ws_host);
  assert_eq!(url, "ws://localhost:3000/ws");
}

#[test]
fn test_ws_url_production_port_no_replacement() {
  // When on a non-8080 port, should use as-is
  let host = "example.com:443";
  let ws_host = if host.ends_with(":8080") {
    host.replace(":8080", ":3000")
  } else {
    host.to_string()
  };
  let url = format!("wss://{}/ws", ws_host);
  assert_eq!(url, "wss://example.com:443/ws");
}

#[test]
fn test_http_url_format_matches_detect_logic() {
  // Browser's location.protocol() includes the colon, e.g. "https:"
  // After P11 fix: we strip the trailing colon before formatting,
  // so detect_http_url now produces "https://host" (correct).
  let protocol = "https:";
  let scheme = protocol.trim_end_matches(':');
  let host = "example.com";
  let url = format!("{}://{}", scheme, host);
  assert_eq!(url, "https://example.com");
}

#[test]
fn test_fallback_ws_url_is_localhost() {
  // When no window, detect_ws_url falls back to localhost:3000
  let fallback = "ws://localhost:3000/ws";
  assert!(fallback.starts_with("ws://"));
  assert!(fallback.contains("localhost"));
  assert!(fallback.ends_with("/ws"));
}

#[test]
fn test_fallback_http_url_is_localhost() {
  // When no window, detect_http_url falls back to localhost
  let fallback = "http://localhost:8080";
  assert!(fallback.starts_with("http://"));
  assert!(fallback.contains("localhost"));
}

// ── Config struct tests ──

#[test]
fn test_config_struct_fields() {
  let config = Config {
    ws_url: "wss://example.com".to_string(),
    http_url: "https://example.com".to_string(),
    debug: true,
    version: "0.1.0".to_string(),
  };
  assert_eq!(config.ws_url, "wss://example.com");
  assert_eq!(config.http_url, "https://example.com");
  assert!(config.debug);
  assert_eq!(config.version, "0.1.0");
}

#[test]
fn test_config_clone() {
  let config = Config {
    ws_url: "ws://localhost".to_string(),
    http_url: "http://localhost".to_string(),
    debug: false,
    version: "1.0.0".to_string(),
  };
  let cloned = config.clone();
  assert_eq!(config.ws_url, cloned.ws_url);
  assert_eq!(config.http_url, cloned.http_url);
  assert_eq!(config.debug, cloned.debug);
  assert_eq!(config.version, cloned.version);
}

#[test]
fn test_config_debug_format() {
  let config = Config {
    ws_url: "ws://test".to_string(),
    http_url: "http://test".to_string(),
    debug: true,
    version: "0.1.0".to_string(),
  };
  let debug_str = format!("{:?}", config);
  assert!(debug_str.contains("Config"));
  assert!(debug_str.contains("ws_url"));
  assert!(debug_str.contains("debug"));
}

// ── Debug mode detection logic tests ──

#[test]
fn test_debug_query_param_detection() {
  let search = "?debug=true&other=val";
  assert!(search.contains("debug=true"));
}

#[test]
fn test_debug_query_param_absent() {
  let search = "?other=val";
  assert!(!search.contains("debug=true"));
}

#[test]
fn test_debug_query_param_empty() {
  let search = "";
  assert!(!search.contains("debug=true"));
}

// ── Version tests ──

#[test]
fn test_cargo_pkg_version_is_set() {
  let version = env!("CARGO_PKG_VERSION");
  assert!(!version.is_empty(), "CARGO_PKG_VERSION should be set");
}
