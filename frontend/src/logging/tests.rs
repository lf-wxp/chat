//! Unit tests for the logging module.
//!
//! Tests cover log level ordering, buffer capacity,
//! per-module filtering, and diagnostic report structure.

use super::*;

// ── LogLevel Ordering ──

#[test]
fn test_log_level_ordering() {
  assert!(LogLevel::Error < LogLevel::Warn);
  assert!(LogLevel::Warn < LogLevel::Info);
  assert!(LogLevel::Info < LogLevel::Debug);
  assert!(LogLevel::Debug < LogLevel::Trace);
}

#[test]
fn test_log_level_display() {
  assert_eq!(LogLevel::Error.to_string(), "ERROR");
  assert_eq!(LogLevel::Warn.to_string(), "WARN");
  assert_eq!(LogLevel::Info.to_string(), "INFO");
  assert_eq!(LogLevel::Debug.to_string(), "DEBUG");
  assert_eq!(LogLevel::Trace.to_string(), "TRACE");
}

// ── LogBuffer Capacity ──

#[test]
fn test_log_buffer_new_capacity() {
  let buf = LogBuffer::new(10);
  assert_eq!(buf.capacity, 10);
  assert!(buf.is_empty());
  assert_eq!(buf.len(), 0);
}

#[test]
fn test_log_buffer_push_within_capacity() {
  let mut buf = LogBuffer::new(3);
  buf.push(LogEntry {
    timestamp: 1,
    level: LogLevel::Info,
    module: "test".to_string(),
    message: "msg1".to_string(),
    data: None,
  });
  assert_eq!(buf.len(), 1);

  buf.push(LogEntry {
    timestamp: 2,
    level: LogLevel::Info,
    module: "test".to_string(),
    message: "msg2".to_string(),
    data: None,
  });
  assert_eq!(buf.len(), 2);
}

#[test]
fn test_log_buffer_eviction_at_capacity() {
  let mut buf = LogBuffer::new(2);

  buf.push(LogEntry {
    timestamp: 1,
    level: LogLevel::Error,
    module: "test".to_string(),
    message: "first".to_string(),
    data: None,
  });
  buf.push(LogEntry {
    timestamp: 2,
    level: LogLevel::Warn,
    module: "test".to_string(),
    message: "second".to_string(),
    data: None,
  });
  buf.push(LogEntry {
    timestamp: 3,
    level: LogLevel::Info,
    module: "test".to_string(),
    message: "third".to_string(),
    data: None,
  });

  assert_eq!(buf.len(), 2);
  let entries = buf.entries();
  assert_eq!(entries[0].message, "second");
  assert_eq!(entries[1].message, "third");
}

#[test]
fn test_log_buffer_clear() {
  let mut buf = LogBuffer::new(5);
  buf.push(LogEntry {
    timestamp: 1,
    level: LogLevel::Info,
    module: "test".to_string(),
    message: "msg".to_string(),
    data: None,
  });
  assert!(!buf.is_empty());
  buf.clear();
  assert!(buf.is_empty());
}

// ── LogBuffer Filtering ──

#[test]
fn test_log_buffer_filter_by_level() {
  let mut buf = LogBuffer::new(10);
  buf.push(LogEntry {
    timestamp: 1,
    level: LogLevel::Error,
    module: "webrtc".to_string(),
    message: "error msg".to_string(),
    data: None,
  });
  buf.push(LogEntry {
    timestamp: 2,
    level: LogLevel::Info,
    module: "chat".to_string(),
    message: "info msg".to_string(),
    data: None,
  });
  buf.push(LogEntry {
    timestamp: 3,
    level: LogLevel::Debug,
    module: "signaling".to_string(),
    message: "debug msg".to_string(),
    data: None,
  });

  // Filter by Warn level (should include Error + Warn)
  let filtered = buf.filter(LogLevel::Warn, &None);
  assert_eq!(filtered.len(), 1);
  assert_eq!(filtered[0].message, "error msg");
}

#[test]
fn test_log_buffer_filter_by_module() {
  let mut buf = LogBuffer::new(10);
  buf.push(LogEntry {
    timestamp: 1,
    level: LogLevel::Error,
    module: "webrtc".to_string(),
    message: "webrtc error".to_string(),
    data: None,
  });
  buf.push(LogEntry {
    timestamp: 2,
    level: LogLevel::Error,
    module: "signaling".to_string(),
    message: "signaling error".to_string(),
    data: None,
  });

  let filtered = buf.filter(LogLevel::Error, &Some("webrtc".to_string()));
  assert_eq!(filtered.len(), 1);
  assert_eq!(filtered[0].module, "webrtc");
}

#[test]
fn test_log_buffer_filter_no_results() {
  let mut buf = LogBuffer::new(10);
  buf.push(LogEntry {
    timestamp: 1,
    level: LogLevel::Info,
    module: "test".to_string(),
    message: "msg".to_string(),
    data: None,
  });

  let filtered = buf.filter(LogLevel::Error, &None);
  assert!(filtered.is_empty());
}

// ── DiagnosticReport ──

#[test]
fn test_diagnostic_config_serialization() {
  let config = DiagnosticConfig {
    debug_mode: true,
    locale: "en".to_string(),
    theme: "dark".to_string(),
    log_buffer_size: 500,
  };
  let json = serde_json::to_string(&config).expect("Should serialize");
  assert!(json.contains("\"debug_mode\":true"));
  assert!(json.contains("\"theme\":\"dark\""));
}

#[test]
fn test_performance_metrics_default() {
  let metrics = PerformanceMetrics {
    page_load_ms: None,
    ws_latency_ms: None,
    memory_usage_bytes: None,
    peer_count: 0,
  };
  assert!(metrics.page_load_ms.is_none());
  assert!(metrics.ws_latency_ms.is_none());
  assert_eq!(metrics.peer_count, 0);
}
