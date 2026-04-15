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

/// Try to initialize the logging system. Returns None if already initialized.
/// Useful for tests where the global subscriber may already be set.
#[cfg(test)]
pub fn try_init(config: &Config) -> anyhow::Result<Option<LogGuard>> {
  // Create log directory if it doesn't exist
  if config.log_output != "stdout" {
    std::fs::create_dir_all(&config.log_dir)?;
  }

  // Parse log level filter
  let env_filter =
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level));

  // Try to initialize based on output configuration
  match config.log_output.as_str() {
    "stdout" => {
      if try_init_stdout_only(config, env_filter) {
        Ok(Some(LogGuard { _file_guard: None }))
      } else {
        Ok(None)
      }
    }
    "file" => {
      let guard = try_init_file_only(config, env_filter)?;
      Ok(guard.map(|g| LogGuard {
        _file_guard: Some(g),
      }))
    }
    _ => {
      // "both" or any other value
      let guard = try_init_both(config, env_filter)?;
      Ok(guard.map(|g| LogGuard {
        _file_guard: Some(g),
      }))
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
// Test Helpers for try_init
// =============================================================================

#[cfg(test)]
fn try_init_stdout_only(config: &Config, env_filter: EnvFilter) -> bool {
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
      .try_init()
      .is_ok()
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
      .try_init()
      .is_ok()
  }
}

#[cfg(test)]
fn try_init_file_only(
  config: &Config,
  env_filter: EnvFilter,
) -> anyhow::Result<Option<tracing_appender::non_blocking::WorkerGuard>> {
  let (non_blocking, guard) = create_file_writer(config)?;

  if config.log_format == "json" {
    if tracing_subscriber::fmt()
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
      .try_init()
      .is_err()
    {
      // Already initialized - return None
      return Ok(None);
    }
  } else {
    if tracing_subscriber::fmt()
      .with_writer(non_blocking)
      .with_ansi(false)
      .compact()
      .with_target(true)
      .with_thread_ids(false)
      .with_file(false)
      .with_line_number(false)
      .with_env_filter(env_filter)
      .try_init()
      .is_err()
    {
      // Already initialized - return None
      return Ok(None);
    }
  }

  Ok(Some(guard))
}

#[cfg(test)]
fn try_init_both(
  config: &Config,
  env_filter: EnvFilter,
) -> anyhow::Result<Option<tracing_appender::non_blocking::WorkerGuard>> {
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

    if tracing_subscriber::registry()
      .with(stdout_layer)
      .with(file_layer)
      .try_init()
      .is_err()
    {
      // Already initialized - return None
      return Ok(None);
    }
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

    if tracing_subscriber::registry()
      .with(stdout_layer)
      .with(file_layer)
      .try_init()
      .is_err()
    {
      // Already initialized - return None
      return Ok(None);
    }
  }

  Ok(Some(guard))
}

// =============================================================================
// Log File Cleanup
// =============================================================================

/// Clean up old log files based on configured limits.
///
/// Enforces two constraints from the server configuration:
/// - `max_files`: Maximum number of log files to retain (oldest removed first)
/// - `max_size_mb`: Maximum total size of all log files in MB (oldest removed first)
///
/// # Errors
///
/// Returns an error if the log directory cannot be read.
pub fn cleanup_old_logs(
  log_dir: &std::path::Path,
  max_files: usize,
  max_size_mb: usize,
) -> std::io::Result<()> {
  if max_files == 0 && max_size_mb == 0 {
    return Ok(());
  }

  let mut log_files: Vec<(std::path::PathBuf, std::fs::Metadata)> = std::fs::read_dir(log_dir)?
    .filter_map(|entry| {
      let entry = entry.ok()?;
      let path = entry.path();
      // Only consider files that look like log files
      let name = path.file_name()?.to_string_lossy().to_string();
      if !name.contains("server.log") {
        return None;
      }
      let meta = entry.metadata().ok()?;
      if meta.is_file() {
        Some((path, meta))
      } else {
        None
      }
    })
    .collect();

  if log_files.is_empty() {
    return Ok(());
  }

  // Sort by modification time, newest first
  log_files.sort_by(|a, b| {
    b.1
      .modified()
      .unwrap_or(std::time::UNIX_EPOCH)
      .cmp(&a.1.modified().unwrap_or(std::time::UNIX_EPOCH))
  });

  // Enforce max_files limit (keep newest, remove oldest)
  if max_files > 0 && log_files.len() > max_files {
    for (path, _) in &log_files[max_files..] {
      let _ = std::fs::remove_file(path);
    }
    log_files.truncate(max_files);
  }

  // Enforce max_size_mb limit (remove oldest files until under limit)
  if max_size_mb > 0 {
    let max_bytes = u64::try_from(max_size_mb).unwrap_or(u64::MAX) * 1024 * 1024;
    let mut total_size: u64 = 0;
    for (path, meta) in &log_files {
      total_size = total_size.saturating_add(meta.len());
      if total_size > max_bytes {
        let _ = std::fs::remove_file(path);
      }
    }
  }

  Ok(())
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

  // Find a valid UTF-8 char boundary at or before max_len to avoid panic
  // when max_len falls within a multi-byte character.
  let mut boundary = max_len;
  while boundary > 0 && !content.is_char_boundary(boundary) {
    boundary -= 1;
  }

  format!(
    "{}... ({} bytes total)",
    &content[..boundary],
    content.len()
  )
}

#[cfg(test)]
mod tests;
