//! Logging system module.
//!
//! This module provides production-grade logging with:
//! - Structured output (JSON for production, pretty for development)
//! - Log rotation (daily/hourly)
//! - Log desensitization (JWT tokens, IPs, passwords)
//! - Async log writing with graceful shutdown

use std::io::IsTerminal;

use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::FmtSpan;

use crate::config::{Config, LogRotation};

/// Worker guard for async log writer.
/// Must be held until application shutdown to ensure all logs are flushed.
pub struct LogGuard {
  /// Guard for non-blocking file writer.
  _file_guard: Option<tracing_appender::non_blocking::WorkerGuard>,
}

/// Initialize the logging system based on configuration.
///
/// # Returns
///
/// Returns a `LogGuard` that must be held until application shutdown
/// to ensure all logs are properly flushed.
///
/// # Errors
///
/// Returns an error if the log directory cannot be created or accessed.
pub fn init(config: &Config) -> anyhow::Result<LogGuard> {
  // Create log directory if it doesn't exist
  if config.log_output != "stdout" {
    std::fs::create_dir_all(&config.log_dir)?;
  }

  // Parse log level filter
  let env_filter =
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level));

  // Initialize based on output configuration
  match config.log_output.as_str() {
    "stdout" => {
      init_stdout_only(config, env_filter);
      Ok(LogGuard { _file_guard: None })
    }
    "file" => {
      let guard = init_file_only(config, env_filter)?;
      Ok(LogGuard {
        _file_guard: Some(guard),
      })
    }
    _ => {
      // "both" or any other value
      let guard = init_both(config, env_filter)?;
      Ok(LogGuard {
        _file_guard: Some(guard),
      })
    }
  }
}

/// Initialize stdout-only logging.
fn init_stdout_only(config: &Config, env_filter: EnvFilter) {
  let is_terminal = std::io::stdout().is_terminal();

  if config.log_format == "json" {
    tracing_subscriber::fmt()
      .with_writer(std::io::stdout)
      .with_ansi(is_terminal)
      .json()
      .with_span_events(FmtSpan::CLOSE)
      .with_current_span(false)
      .with_target(true)
      .with_thread_ids(false)
      .with_thread_names(false)
      .with_file(false)
      .with_line_number(false)
      .with_env_filter(env_filter)
      .init();
  } else {
    tracing_subscriber::fmt()
      .with_writer(std::io::stdout)
      .with_ansi(is_terminal)
      .pretty()
      .with_target(true)
      .with_thread_ids(false)
      .with_file(true)
      .with_line_number(true)
      .with_env_filter(env_filter)
      .init();
  }
}

/// Initialize file-only logging.
fn init_file_only(
  config: &Config,
  env_filter: EnvFilter,
) -> anyhow::Result<tracing_appender::non_blocking::WorkerGuard> {
  let (non_blocking, guard) = create_file_writer(config)?;

  if config.log_format == "json" {
    tracing_subscriber::fmt()
      .with_writer(non_blocking)
      .with_ansi(false)
      .json()
      .with_span_events(FmtSpan::CLOSE)
      .with_current_span(false)
      .with_target(true)
      .with_thread_ids(false)
      .with_thread_names(false)
      .with_file(false)
      .with_line_number(false)
      .with_env_filter(env_filter)
      .init();
  } else {
    tracing_subscriber::fmt()
      .with_writer(non_blocking)
      .with_ansi(false)
      .compact()
      .with_target(true)
      .with_thread_ids(false)
      .with_file(false)
      .with_line_number(false)
      .with_env_filter(env_filter)
      .init();
  }

  Ok(guard)
}

/// Initialize both stdout and file logging.
fn init_both(
  config: &Config,
  env_filter: EnvFilter,
) -> anyhow::Result<tracing_appender::non_blocking::WorkerGuard> {
  use tracing_subscriber::Layer;
  use tracing_subscriber::layer::SubscriberExt;
  use tracing_subscriber::util::SubscriberInitExt;

  let is_terminal = std::io::stdout().is_terminal();
  let (file_writer, guard) = create_file_writer(config)?;

  if config.log_format == "json" {
    let stdout_layer = tracing_subscriber::fmt::layer()
      .with_writer(std::io::stdout)
      .with_ansi(is_terminal)
      .json()
      .with_span_events(FmtSpan::CLOSE)
      .with_target(true)
      .with_file(false)
      .with_line_number(false)
      .with_filter(env_filter.clone());

    let file_layer = tracing_subscriber::fmt::layer()
      .with_writer(file_writer)
      .with_ansi(false)
      .json()
      .with_span_events(FmtSpan::CLOSE)
      .with_target(true)
      .with_file(false)
      .with_line_number(false)
      .with_filter(env_filter);

    tracing_subscriber::registry()
      .with(stdout_layer)
      .with(file_layer)
      .init();
  } else {
    let stdout_layer = tracing_subscriber::fmt::layer()
      .with_writer(std::io::stdout)
      .with_ansi(is_terminal)
      .pretty()
      .with_target(true)
      .with_file(true)
      .with_line_number(true)
      .with_filter(env_filter.clone());

    let file_layer = tracing_subscriber::fmt::layer()
      .with_writer(file_writer)
      .with_ansi(false)
      .compact()
      .with_target(true)
      .with_file(false)
      .with_line_number(false)
      .with_filter(env_filter);

    tracing_subscriber::registry()
      .with(stdout_layer)
      .with(file_layer)
      .init();
  }

  Ok(guard)
}

/// Create a non-blocking file writer with rotation.
fn create_file_writer(
  config: &Config,
) -> anyhow::Result<(
  tracing_appender::non_blocking::NonBlocking,
  tracing_appender::non_blocking::WorkerGuard,
)> {
  let log_dir = config.log_dir.clone();

  // Create rolling file appender based on rotation strategy
  let file_appender = match config.log_rotation {
    LogRotation::Daily => tracing_appender::rolling::daily(&log_dir, "server.log"),
    LogRotation::Hourly => tracing_appender::rolling::hourly(&log_dir, "server.log"),
    LogRotation::Never => tracing_appender::rolling::never(&log_dir, "server.log"),
  };

  // Create non-blocking writer for async logging
  Ok(tracing_appender::non_blocking(file_appender))
}

// =============================================================================
// Log Desensitization Utilities
// =============================================================================

/// Desensitize a JWT token by showing only first 8 and last 4 characters.
///
/// # Example
///
/// ```ignore
/// let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
/// let desensitized = desensitize_jwt(token);
/// // Result: "eyJhbGci******hsR8U"
/// ```
#[must_use]
pub fn desensitize_jwt(token: &str) -> String {
  if token.len() <= 12 {
    return "****".to_string();
  }

  let first_part = &token[..8];
  let last_part = &token[token.len() - 4..];
  format!("{first_part}****{last_part}")
}

/// Mask an IP address by hiding the last octet.
///
/// # Example
///
/// ```ignore
/// let ip = "192.168.1.100";
/// let masked = mask_ip(ip);
/// // Result: "192.168.1.xxx"
/// ```
#[must_use]
pub fn mask_ip(ip: &str) -> String {
  // Handle IPv4
  if let Some(last_dot) = ip.rfind('.') {
    return format!("{}xxx", &ip[..=last_dot]);
  }

  // Handle IPv6 (simplified masking)
  if ip.contains(':')
    && let Some(last_colon) = ip.rfind(':')
  {
    return format!("{}xxxx", &ip[..=last_colon]);
  }

  // Unknown format, return masked
  "xxx.xxx.xxx.xxx".to_string()
}

/// Desensitize a password by always showing asterisks.
#[must_use]
pub const fn desensitize_password() -> &'static str {
  "********"
}

/// Create a summary of a message for logging (avoid logging full content).
#[must_use]
pub fn summarize_message(content: &str, max_len: usize) -> String {
  if content.len() <= max_len {
    return content.to_string();
  }

  format!("{}... ({} bytes total)", &content[..max_len], content.len())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_desensitize_jwt() {
    let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
    let result = desensitize_jwt(token);
    assert!(result.starts_with("eyJhbGci"));
    assert!(result.ends_with("sR8U"));
    assert!(result.contains("****"));

    // Short token
    let short_token = "short";
    let result = desensitize_jwt(short_token);
    assert_eq!(result, "****");
  }

  #[test]
  fn test_mask_ip_ipv4() {
    let ip = "192.168.1.100";
    let masked = mask_ip(ip);
    assert_eq!(masked, "192.168.1.xxx");
  }

  #[test]
  fn test_mask_ip_ipv6() {
    let ip = "2001:0db8:85a3:0000:0000:8a2e:0370:7334";
    let masked = mask_ip(ip);
    assert!(masked.ends_with("xxxx"));
  }

  #[test]
  fn test_desensitize_password() {
    assert_eq!(desensitize_password(), "********");
  }

  #[test]
  fn test_summarize_message() {
    let content = "This is a very long message that needs to be summarized";
    let result = summarize_message(content, 20);
    assert!(result.starts_with("This is a very long"));
    assert!(result.contains("..."));
    assert!(result.contains(&format!("{} bytes", content.len())));
  }

  #[test]
  fn test_summarize_message_short() {
    let content = "Short";
    let result = summarize_message(content, 20);
    assert_eq!(result, "Short");
  }
}
