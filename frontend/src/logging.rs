//! Frontend logging module.
//!
//! Provides structured client-side logging with:
//! - Configurable log levels (error/warn/info/debug/trace)
//! - Per-module filtering via localStorage
//! - In-memory ring buffer (last N entries)
//! - Debug panel integration
//! - Diagnostic report generation

use leptos::prelude::*;
use std::collections::VecDeque;
use wasm_bindgen::JsCast;
use web_sys::console;

/// Log level enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[repr(u8)]
pub enum LogLevel {
  /// Error level (always shown)
  Error = 0,
  /// Warning level
  Warn = 1,
  /// Info level
  Info = 2,
  /// Debug level
  Debug = 3,
  /// Trace level (most verbose)
  Trace = 4,
}

impl std::fmt::Display for LogLevel {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Error => write!(f, "ERROR"),
      Self::Warn => write!(f, "WARN"),
      Self::Info => write!(f, "INFO"),
      Self::Debug => write!(f, "DEBUG"),
      Self::Trace => write!(f, "TRACE"),
    }
  }
}

/// A single log entry.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LogEntry {
  /// Timestamp (unix ms)
  pub timestamp: i64,
  /// Log level
  pub level: LogLevel,
  /// Module / source tag
  pub module: String,
  /// Human-readable message
  pub message: String,
  /// Optional structured data (JSON string)
  pub data: Option<String>,
}

/// Ring buffer for in-memory log storage.
#[derive(Debug, Clone)]
pub struct LogBuffer {
  /// Buffer entries
  entries: VecDeque<LogEntry>,
  /// Maximum capacity
  pub capacity: usize,
}

impl LogBuffer {
  /// Create a new ring buffer with the given capacity.
  #[must_use]
  pub fn new(capacity: usize) -> Self {
    Self {
      entries: VecDeque::with_capacity(capacity),
      capacity,
    }
  }

  /// Push a log entry, evicting oldest if at capacity.
  pub fn push(&mut self, entry: LogEntry) {
    if self.entries.len() >= self.capacity {
      self.entries.pop_front();
    }
    self.entries.push_back(entry);
  }

  /// Get all entries as a Vec (oldest first).
  #[must_use]
  pub fn entries(&self) -> Vec<LogEntry> {
    self.entries.iter().cloned().collect()
  }

  /// Filter entries by level and module.
  ///
  /// Supports comma-separated multi-module filters (e.g., `"webrtc,signaling"`).
  /// When `module_filter` is `Some`, a log entry passes if its module contains
  /// any of the comma-separated filter segments.
  #[must_use]
  pub fn filter(&self, min_level: LogLevel, module_filter: &Option<String>) -> Vec<LogEntry> {
    self
      .entries
      .iter()
      .filter(|e| e.level <= min_level)
      .filter(|e| {
        module_filter.as_ref().is_none_or(|f| {
          f.split(',').any(|segment| e.module.contains(segment.trim()))
        })
      })
      .cloned()
      .collect()
  }

  /// Clear all entries.
  pub fn clear(&mut self) {
    self.entries.clear();
  }

  /// Current number of entries.
  #[must_use]
  pub fn len(&self) -> usize {
    self.entries.len()
  }

  /// Whether the buffer is empty.
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.entries.is_empty()
  }
}

/// Global logger state (held in Leptos context).
#[derive(Debug, Clone, Copy)]
pub struct LoggerState {
  /// Ring buffer
  pub buffer: RwSignal<LogBuffer>,
  /// Minimum console log level (non-debug mode)
  pub console_min_level: RwSignal<LogLevel>,
  /// Per-module filter (comma-separated, e.g., "webrtc,signaling")
  pub module_filter: RwSignal<Option<String>>,
}

impl LoggerState {
  /// Create new logger state.
  #[must_use]
  pub fn new() -> Self {
    let buffer_size = Self::load_buffer_size();
    Self {
      buffer: RwSignal::new(LogBuffer::new(buffer_size)),
      console_min_level: RwSignal::new(LogLevel::Warn),
      module_filter: RwSignal::new(Self::load_module_filter()),
    }
  }

  /// Maximum number of recent errors in diagnostic report.
  const MAX_RECENT_ERRORS: usize = 50;

  /// Log a message to the ring buffer and console.
  ///
  /// The ring buffer always records all log entries regardless of module filter.
  /// The module filter only affects console output, so that the debug panel and
  /// diagnostic reports retain the full log history.
  pub fn log(&self, level: LogLevel, module: &str, message: &str, data: Option<&str>) {
    let entry = LogEntry {
      timestamp: chrono::Utc::now().timestamp_millis(),
      level,
      module: module.to_string(),
      message: message.to_string(),
      data: data.map(String::from),
    };

    // Always push to ring buffer (module filter does NOT affect storage)
    self.buffer.update(|buf| buf.push(entry.clone()));

    // Console output based on mode and module filter
    let console_level = self.console_min_level.get();
    if level <= console_level && self.passes_module_filter(module) {
      let formatted = format!("[{}][{}] {}", entry.level, entry.module, entry.message);
      match level {
        LogLevel::Error => console::error_1(&formatted.into()),
        LogLevel::Warn => console::warn_1(&formatted.into()),
        LogLevel::Info => console::info_1(&formatted.into()),
        LogLevel::Debug => console::log_1(&formatted.into()),
        LogLevel::Trace => console::log_1(&formatted.into()),
      }
    }
  }

  /// Check whether a module name passes the current module filter.
  ///
  /// Supports comma-separated multi-module filters, e.g. `"webrtc,signaling"`.
  /// Returns `true` if no filter is set or if the module matches any filter segment.
  fn passes_module_filter(&self, module: &str) -> bool {
    match self.module_filter.get() {
      Some(filter) => filter
        .split(',')
        .any(|segment| module.contains(segment.trim())),
      None => true,
    }
  }

  /// Convenience methods for each level.
  pub fn error(&self, module: &str, message: &str, data: Option<&str>) {
    self.log(LogLevel::Error, module, message, data);
  }

  pub fn warn(&self, module: &str, message: &str, data: Option<&str>) {
    self.log(LogLevel::Warn, module, message, data);
  }

  pub fn info(&self, module: &str, message: &str, data: Option<&str>) {
    self.log(LogLevel::Info, module, message, data);
  }

  pub fn debug(&self, module: &str, message: &str, data: Option<&str>) {
    self.log(LogLevel::Debug, module, message, data);
  }

  pub fn trace(&self, module: &str, message: &str, data: Option<&str>) {
    self.log(LogLevel::Trace, module, message, data);
  }

  /// Update console log level based on debug mode.
  pub fn set_debug_mode(&self, enabled: bool) {
    self.console_min_level.set(if enabled {
      LogLevel::Trace
    } else {
      LogLevel::Warn
    });
  }

  /// Load buffer size from localStorage.
  fn load_buffer_size() -> usize {
    crate::utils::load_from_local_storage("debug_buffer_size")
      .and_then(|v| v.parse().ok())
      .unwrap_or(1000)
  }

  /// Load module filter from localStorage.
  fn load_module_filter() -> Option<String> {
    crate::utils::load_from_local_storage("debug_filter")
  }
}

impl Default for LoggerState {
  fn default() -> Self {
    Self::new()
  }
}

// ── Context helpers ──

/// Provide LoggerState to the Leptos component tree.
pub fn provide_logger_state() -> LoggerState {
  let state = LoggerState::new();
  provide_context(state);
  state
}

/// Retrieve LoggerState from the Leptos context.
///
/// # Panics
/// Panics if LoggerState has not been provided.
#[must_use]
pub fn use_logger_state() -> LoggerState {
  expect_context::<LoggerState>()
}

// ── Diagnostic Report ──

/// Diagnostic report data structure.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DiagnosticReport {
  /// Report generation timestamp
  pub timestamp: String,
  /// Browser user agent
  pub user_agent: String,
  /// Connection status
  pub connected: bool,
  /// Performance metrics
  pub performance: PerformanceMetrics,
  /// Recent error logs (last 50)
  pub recent_errors: Vec<LogEntry>,
  /// Current configuration (non-sensitive)
  pub configuration: DiagnosticConfig,
}

/// Performance metrics for diagnostic report.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PerformanceMetrics {
  /// Page load time (ms)
  pub page_load_ms: Option<f64>,
  /// WebSocket latency (ms, estimated)
  pub ws_latency_ms: Option<f64>,
  /// Memory usage (bytes, if available)
  pub memory_usage_bytes: Option<f64>,
  /// Number of active peer connections
  pub peer_count: usize,
}

/// Non-sensitive configuration for diagnostic report.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DiagnosticConfig {
  /// Debug mode status
  pub debug_mode: bool,
  /// Locale
  pub locale: String,
  /// Theme
  pub theme: String,
  /// Log buffer size
  pub log_buffer_size: usize,
}

/// Generate a diagnostic report JSON object.
///
/// This function collects browser information, connection status,
/// performance metrics, recent error logs, and current configuration.
/// It intentionally excludes sensitive data such as JWT tokens,
/// user credentials, and message content.
pub fn generate_diagnostic_report(logger: &LoggerState, app_state: &AppState) -> DiagnosticReport {
  let window = web_sys::window();
  let navigator = window.as_ref().map(|w| w.navigator());

  let user_agent = navigator
    .and_then(|n| n.user_agent().ok())
    .unwrap_or_default();

  let connected = app_state.connected.get();

  let performance = PerformanceMetrics {
    page_load_ms: window.as_ref().and_then(|w| {
      let perf = w.performance()?;
      let nav = perf.get_entries_by_type("navigation");
      let entry = nav.get(0);
      let load_event_end = js_sys::Reflect::get(&entry, &"loadEventEnd".into()).ok()?;
      load_event_end.as_f64()
    }),
    ws_latency_ms: None, // Populated dynamically
    memory_usage_bytes: window.as_ref().and_then(|w| {
      js_sys::Reflect::get(
        &w.performance()
          .map(|p| p.into())
          .unwrap_or(wasm_bindgen::JsValue::UNDEFINED),
        &"memory".into(),
      )
      .ok()
      .and_then(|m| js_sys::Reflect::get(&m, &"usedJSHeapSize".into()).ok())
      .and_then(|v| v.as_f64())
    }),
    peer_count: app_state.network_quality.get().len(),
  };

  let recent_errors: Vec<LogEntry> = logger
    .buffer
    .get()
    .filter(LogLevel::Error, &None)
    .into_iter()
    .rev()
    .take(LoggerState::MAX_RECENT_ERRORS)
    .collect::<Vec<_>>()
    .into_iter()
    .rev()
    .collect();

  let configuration = DiagnosticConfig {
    debug_mode: app_state.debug.get(),
    locale: app_state.locale.get(),
    theme: app_state.theme.get(),
    log_buffer_size: logger.buffer.get().capacity,
  };

  DiagnosticReport {
    timestamp: chrono::Utc::now().to_rfc3339(),
    user_agent,
    connected,
    performance,
    recent_errors,
    configuration,
  }
}

/// Download diagnostic report as a JSON file.
pub fn download_diagnostic_report(logger: &LoggerState, app_state: &AppState) {
  let report = generate_diagnostic_report(logger, app_state);
  if let Ok(json) = serde_json::to_string_pretty(&report) {
    let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
    let filename = format!("diagnostic-{}.json", timestamp);
    // Use browser download via creating a blob link
    if let Some(window) = web_sys::window()
      && let Some(document) = window.document()
    {
      let blob_opts = web_sys::BlobPropertyBag::new();
      blob_opts.set_type("application/json");
      let blob = web_sys::Blob::new_with_str_sequence_and_options(
        &js_sys::Array::of1(&json.into()),
        &blob_opts,
      )
      .ok();
      if let Some(blob) = blob
        && let Some(url) = web_sys::Url::create_object_url_with_blob(&blob).ok()
        && let Ok(link) = document.create_element("a")
      {
        let link: web_sys::HtmlElement = link.unchecked_into();
        link.set_attribute("href", &url).ok();
        link.set_attribute("download", &filename).ok();
        link.click();
        let _ = web_sys::Url::revoke_object_url(&url);
      }
    }
  }
}

// ── Re-export AppState from state module ──
use crate::state::AppState;

#[cfg(test)]
mod tests;
