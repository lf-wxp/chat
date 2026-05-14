//! Server configuration module.
//!
//! This module provides configuration management for the signaling server,
//! supporting environment variable based configuration with sensible defaults.

use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

/// Log rotation strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogRotation {
  /// Rotate logs daily (default).
  Daily,
  /// Rotate logs hourly.
  Hourly,
  /// Never rotate logs.
  Never,
}

impl std::str::FromStr for LogRotation {
  type Err = std::convert::Infallible;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_lowercase().as_str() {
      "hourly" => Ok(Self::Hourly),
      "never" => Ok(Self::Never),
      _ => Ok(Self::Daily),
    }
  }
}

/// TLS configuration.
#[derive(Debug, Clone)]
pub struct TlsConfig {
  /// Path to TLS certificate file.
  pub cert_path: PathBuf,
  /// Path to TLS private key file.
  pub key_path: PathBuf,
}

/// Server configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
  // Network configuration
  /// Server listening address.
  pub addr: SocketAddr,

  // Security configuration
  /// JWT secret key for token signing and verification.
  pub jwt_secret: String,

  // ICE configuration
  /// STUN/TURN servers configuration for WebRTC.
  ///
  /// Pushed to every authenticated client inside `AuthSuccess`. The
  /// list is the *base* list — the runtime auto-prepends an entry
  /// pointing at the embedded STUN service (see [`Self::stun_port`])
  /// so a fresh deployment serves WebRTC peers on a closed LAN with
  /// **zero configuration**.
  pub ice_servers: Vec<String>,

  /// UDP port for the embedded STUN service.
  ///
  /// `Some(port)` — bind on `0.0.0.0:port`. The default `3478` is
  /// the IANA-assigned STUN port; pick any other value if the host
  /// already runs another STUN/coturn instance.
  ///
  /// `None` — disable the embedded STUN entirely. Use this when
  /// deploying behind a load balancer that already forwards to a
  /// dedicated STUN/TURN cluster.
  ///
  /// Configured via the `STUN_PORT` environment variable
  /// (`STUN_PORT=0` ⇒ disabled).
  pub stun_port: Option<u16>,

  // TLS configuration
  /// Optional TLS configuration for secure connections.
  pub tls: Option<TlsConfig>,

  // Static files configuration
  /// Static files directory for serving frontend.
  pub static_dir: PathBuf,
  /// Sticker resources directory.
  pub stickers_dir: PathBuf,

  // Logging configuration
  /// Log level (trace, debug, info, warn, error).
  pub log_level: String,
  /// Log format: "pretty" (development) or "json" (production).
  pub log_format: String,
  /// Log output: "stdout", "file", or "both".
  pub log_output: String,
  /// Log rotation strategy.
  pub log_rotation: LogRotation,
  /// Log directory for file output.
  pub log_dir: PathBuf,
  /// Maximum number of log files to keep.
  pub log_max_files: usize,
  /// Maximum log directory size in MB.
  pub log_max_size_mb: usize,

  // WebSocket configuration
  /// Heartbeat interval for WebSocket connections.
  pub heartbeat_interval: Duration,
  /// Timeout for heartbeat response.
  pub heartbeat_timeout: Duration,
  /// Maximum WebSocket message size in bytes.
  pub max_message_size: usize,
  /// Maximum pending messages in the send queue per connection.
  pub send_queue_size: usize,
}

impl Config {
  /// Load configuration from environment variables.
  ///
  /// # Errors
  ///
  /// Returns an error if required environment variables are missing or invalid.
  pub fn from_env() -> anyhow::Result<Self> {
    // Network configuration
    let port = env::var("PORT")
      .unwrap_or_else(|_| "3000".to_string())
      .parse()
      .unwrap_or(3000);

    let addr: SocketAddr = ([0, 0, 0, 0], port).into();

    // Security configuration
    let jwt_secret =
      env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret-change-in-production".to_string());

    // Embedded STUN port. `STUN_PORT=0` disables the embedded
    // service so an external coturn deployment can take over.
    let stun_port = match env::var("STUN_PORT") {
      Ok(s) => match s.trim().parse::<u16>() {
        Ok(0) => None,
        Ok(p) => Some(p),
        Err(_) => Some(3478),
      },
      Err(_) => Some(3478),
    };

    // ICE servers configuration
    //
    // `STUN_TURN_SERVERS` is a comma-separated list of ICE URLs.
    //
    // **Zero-config behaviour** (env var unset): we synthesise a
    // list that works on **both** a closed LAN (the embedded STUN
    // entry, auto-discovered host IP) and the open internet (Google
    // public STUN as a fallback). The browser ICE agent queries all
    // entries in parallel and uses whichever responds first, so a
    // mismatched entry is harmless — it just times out.
    //
    // An *empty* value (`STUN_TURN_SERVERS=`) is treated as "no ICE
    // servers" rather than `vec![""]`, which matters for E2E tests
    // that run both peers on `127.0.0.1` — host candidates alone
    // suffice and reaching out to a public STUN server stalls ICE
    // gathering in sandboxed CI environments.
    //
    // An explicitly-provided non-empty list overrides the default
    // entirely so production deployments can point at a dedicated
    // STUN/TURN cluster without inheriting the embedded entries.
    let ice_servers = env::var("STUN_TURN_SERVERS").map_or_else(
      |_| {
        let mut list = Vec::new();
        if let Some(port) = stun_port {
          let lan = crate::stun::detect_lan_ipv4();
          list.push(format!("stun:{lan}:{port}"));
        }
        list.push("stun:stun.l.google.com:19302".to_string());
        list.push("stun:stun1.l.google.com:19302".to_string());
        list
      },
      |s| {
        s.split(',')
          .map(str::trim)
          .filter(|part| !part.is_empty())
          .map(String::from)
          .collect()
      },
    );

    // TLS configuration
    let tls = match (env::var("TLS_CERT_PATH"), env::var("TLS_KEY_PATH")) {
      (Ok(cert_path), Ok(key_path)) => Some(TlsConfig {
        cert_path: PathBuf::from(cert_path),
        key_path: PathBuf::from(key_path),
      }),
      _ => None,
    };

    // Static files configuration
    let static_dir =
      env::var("STATIC_DIR").map_or_else(|_| PathBuf::from("../frontend/dist"), PathBuf::from);

    let stickers_dir =
      env::var("STICKERS_DIR").map_or_else(|_| PathBuf::from("./assets/stickers"), PathBuf::from);

    // Logging configuration
    let log_level = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let log_format = env::var("RUST_LOG_FORMAT").unwrap_or_else(|_| "pretty".to_string());
    let log_output = env::var("LOG_OUTPUT").unwrap_or_else(|_| "both".to_string());
    let log_rotation = env::var("LOG_ROTATION")
      .unwrap_or_else(|_| "daily".to_string())
      .parse()
      .unwrap_or(LogRotation::Daily);

    let log_dir = env::var("LOG_DIR").map_or_else(|_| PathBuf::from("./logs"), PathBuf::from);

    let log_max_files = env::var("LOG_MAX_FILES")
      .map(|s| s.parse().unwrap_or(30))
      .unwrap_or(30);

    let log_max_size_mb = env::var("LOG_MAX_SIZE_MB")
      .map(|s| s.parse().unwrap_or(500))
      .unwrap_or(500);

    // WebSocket configuration
    let heartbeat_interval_secs = env::var("HEARTBEAT_INTERVAL_SECS")
      .map(|s| s.parse().unwrap_or(30))
      .unwrap_or(30);

    let heartbeat_timeout_secs = env::var("HEARTBEAT_TIMEOUT_SECS")
      .map(|s| s.parse().unwrap_or(60))
      .unwrap_or(60);

    let max_message_size = env::var("MAX_MESSAGE_SIZE")
      .map(|s| s.parse().unwrap_or(1024 * 1024)) // 1MB default
      .unwrap_or(1024 * 1024);

    let send_queue_size = env::var("SEND_QUEUE_SIZE")
      .map(|s| s.parse().unwrap_or(256))
      .unwrap_or(256);

    Ok(Self {
      addr,
      jwt_secret,
      ice_servers,
      stun_port,
      tls,
      static_dir,
      stickers_dir,
      log_level,
      log_format,
      log_output,
      log_rotation,
      log_dir,
      log_max_files,
      log_max_size_mb,
      heartbeat_interval: Duration::from_secs(heartbeat_interval_secs),
      heartbeat_timeout: Duration::from_secs(heartbeat_timeout_secs),
      max_message_size,
      send_queue_size,
    })
  }
}

impl Default for Config {
  fn default() -> Self {
    Self::from_env().expect("failed to create default config")
  }
}

#[cfg(test)]
mod tests;
