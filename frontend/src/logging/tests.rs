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

// ── Multi-module filtering tests ──

#[test]
fn test_log_buffer_filter_multi_module() {
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
  buf.push(LogEntry {
    timestamp: 3,
    level: LogLevel::Error,
    module: "auth".to_string(),
    message: "auth error".to_string(),
    data: None,
  });

  // Multi-module filter: "webrtc,signaling" should match 2 of 3
  let filtered = buf.filter(LogLevel::Error, &Some("webrtc,signaling".to_string()));
  assert_eq!(filtered.len(), 2);
  assert!(filtered.iter().any(|e| e.module == "webrtc"));
  assert!(filtered.iter().any(|e| e.module == "signaling"));
}

#[test]
fn test_log_buffer_filter_multi_module_with_whitespace() {
  let mut buf = LogBuffer::new(10);
  buf.push(LogEntry {
    timestamp: 1,
    level: LogLevel::Error,
    module: "webrtc".to_string(),
    message: "msg".to_string(),
    data: None,
  });
  buf.push(LogEntry {
    timestamp: 2,
    level: LogLevel::Error,
    module: "signaling".to_string(),
    message: "msg".to_string(),
    data: None,
  });

  // Whitespace around segments should be trimmed
  let filtered = buf.filter(LogLevel::Error, &Some(" webrtc , signaling ".to_string()));
  assert_eq!(filtered.len(), 2);
}

#[test]
fn test_log_buffer_filter_none_module_returns_all() {
  let mut buf = LogBuffer::new(10);
  buf.push(LogEntry {
    timestamp: 1,
    level: LogLevel::Info,
    module: "webrtc".to_string(),
    message: "msg1".to_string(),
    data: None,
  });
  buf.push(LogEntry {
    timestamp: 2,
    level: LogLevel::Info,
    module: "auth".to_string(),
    message: "msg2".to_string(),
    data: None,
  });

  let filtered = buf.filter(LogLevel::Trace, &None);
  assert_eq!(filtered.len(), 2);
}

// ── LogBuffer overflow tests ──

#[test]
fn test_log_buffer_capacity_one() {
  let mut buf = LogBuffer::new(1);
  buf.push(LogEntry {
    timestamp: 1,
    level: LogLevel::Info,
    module: "test".to_string(),
    message: "first".to_string(),
    data: None,
  });
  buf.push(LogEntry {
    timestamp: 2,
    level: LogLevel::Info,
    module: "test".to_string(),
    message: "second".to_string(),
    data: None,
  });
  assert_eq!(buf.len(), 1);
  assert_eq!(buf.entries()[0].message, "second");
}

#[test]
fn test_log_buffer_heavy_overflow() {
  let mut buf = LogBuffer::new(5);
  for i in 0..100 {
    buf.push(LogEntry {
      timestamp: i,
      level: LogLevel::Info,
      module: "test".to_string(),
      message: format!("msg-{}", i),
      data: None,
    });
  }
  assert_eq!(buf.len(), 5);
  let entries = buf.entries();
  // Should contain the last 5 entries (95..100)
  assert_eq!(entries[0].message, "msg-95");
  assert_eq!(entries[4].message, "msg-99");
}

// ── LogEntry data field tests ──

#[test]
fn test_log_entry_with_data() {
  let entry = LogEntry {
    timestamp: 12345,
    level: LogLevel::Debug,
    module: "test".to_string(),
    message: "something happened".to_string(),
    data: Some(r#"{"key":"value"}"#.to_string()),
  };
  assert_eq!(entry.timestamp, 12345);
  assert_eq!(entry.level, LogLevel::Debug);
  assert!(entry.data.is_some());
  assert!(entry.data.as_ref().unwrap().contains("key"));
}

#[test]
fn test_log_entry_serialization() {
  let entry = LogEntry {
    timestamp: 1000,
    level: LogLevel::Warn,
    module: "signaling".to_string(),
    message: "test message".to_string(),
    data: None,
  };
  let json = serde_json::to_string(&entry).expect("Should serialize LogEntry");
  // serde serializes enum variants as strings by default (e.g. "Warn")
  assert!(json.contains("\"level\":\"Warn\""));
  assert!(json.contains("\"module\":\"signaling\""));
  assert!(json.contains("\"message\":\"test message\""));
}

#[test]
fn test_log_entry_clone() {
  let entry = LogEntry {
    timestamp: 1,
    level: LogLevel::Error,
    module: "test".to_string(),
    message: "original".to_string(),
    data: Some("data".to_string()),
  };
  let cloned = entry.clone();
  assert_eq!(entry.timestamp, cloned.timestamp);
  assert_eq!(entry.level, cloned.level);
  assert_eq!(entry.module, cloned.module);
  assert_eq!(entry.message, cloned.message);
  assert_eq!(entry.data, cloned.data);
}

// ── LogLevel repr tests ──

#[test]
fn test_log_level_repr_values() {
  assert_eq!(LogLevel::Error as u8, 0);
  assert_eq!(LogLevel::Warn as u8, 1);
  assert_eq!(LogLevel::Info as u8, 2);
  assert_eq!(LogLevel::Debug as u8, 3);
  assert_eq!(LogLevel::Trace as u8, 4);
}

// ── DiagnosticReport/Config serialization tests ──

#[test]
fn test_diagnostic_config_default_values() {
  let config = DiagnosticConfig {
    debug_mode: false,
    locale: "zh-CN".to_string(),
    theme: "system".to_string(),
    log_buffer_size: 1000,
  };
  let json = serde_json::to_string(&config).expect("Should serialize");
  assert!(json.contains("\"debug_mode\":false"));
  assert!(json.contains("\"locale\":\"zh-CN\""));
  assert!(json.contains("\"log_buffer_size\":1000"));
}

#[test]
fn test_performance_metrics_with_values() {
  let metrics = PerformanceMetrics {
    page_load_ms: Some(1234.5),
    ws_latency_ms: Some(42.0),
    memory_usage_bytes: Some(1048576.0),
    peer_count: 3,
  };
  let json = serde_json::to_string(&metrics).expect("Should serialize");
  assert!(json.contains("\"peer_count\":3"));
  assert!(json.contains("1234.5"));
}

#[test]
fn test_max_recent_errors_constant() {
  assert_eq!(LoggerState::MAX_RECENT_ERRORS, 50);
}

// ── Filter combined level + module tests ──

#[test]
fn test_log_buffer_filter_combined_level_and_module() {
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
    level: LogLevel::Info,
    module: "webrtc".to_string(),
    message: "webrtc info".to_string(),
    data: None,
  });
  buf.push(LogEntry {
    timestamp: 3,
    level: LogLevel::Error,
    module: "auth".to_string(),
    message: "auth error".to_string(),
    data: None,
  });

  // Filter: errors only + webrtc module
  let filtered = buf.filter(LogLevel::Error, &Some("webrtc".to_string()));
  assert_eq!(filtered.len(), 1);
  assert_eq!(filtered[0].message, "webrtc error");
}
