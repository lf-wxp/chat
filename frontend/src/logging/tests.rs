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

// ── Diagnostic report privacy whitelist ──
//
// The diagnostic report is downloaded by users and frequently attached
// to bug reports, so it MUST NEVER leak sensitive information. These
// tests pin the data-shape contract so that any future field added to
// `DiagnosticReport`, `PerformanceMetrics`, or `DiagnosticConfig`
// forces the developer to explicitly acknowledge it here.

/// Exhaustive list of top-level JSON keys the report may contain.
/// Adding a new field requires updating this list AND reviewing it
/// for privacy implications (see requirements.md → Observability →
/// Diagnostic Report).
const ALLOWED_REPORT_KEYS: &[&str] = &[
  "timestamp",
  "user_agent",
  "connected",
  "performance",
  "recent_errors",
  "configuration",
];

/// Fields inside `PerformanceMetrics` — purely numeric metrics.
const ALLOWED_PERFORMANCE_KEYS: &[&str] = &[
  "page_load_ms",
  "ws_latency_ms",
  "memory_usage_bytes",
  "peer_count",
];

/// Fields inside `DiagnosticConfig` — user-facing, non-sensitive
/// preferences only.
const ALLOWED_CONFIG_KEYS: &[&str] = &["debug_mode", "locale", "theme", "log_buffer_size"];

/// Substrings that, if present in any struct field name, indicate a
/// privacy regression. This is a defence-in-depth guard — the
/// whitelist above is the primary contract.
const SENSITIVE_FIELD_SUBSTRINGS: &[&str] = &[
  "token",
  "password",
  "secret",
  "jwt",
  "credential",
  "session_id",
  "api_key",
  "private_key",
  "shared_secret",
];

/// Recursively collect every JSON object key from a `serde_json::Value`.
fn collect_json_keys(value: &serde_json::Value, out: &mut std::collections::BTreeSet<String>) {
  match value {
    serde_json::Value::Object(map) => {
      for (k, v) in map {
        out.insert(k.clone());
        collect_json_keys(v, out);
      }
    }
    serde_json::Value::Array(items) => {
      for item in items {
        collect_json_keys(item, out);
      }
    }
    _ => {}
  }
}

#[test]
fn diagnostic_report_top_level_fields_are_whitelisted() {
  // Construct a fully populated report so serde_json::to_value emits
  // every field (Option::None fields are still serialised with null
  // under serde defaults, but we populate them anyway for rigour).
  let report = DiagnosticReport {
    timestamp: "2026-05-09T00:00:00Z".to_string(),
    user_agent: "TestAgent/1.0".to_string(),
    connected: true,
    performance: PerformanceMetrics {
      page_load_ms: Some(1.0),
      ws_latency_ms: Some(2.0),
      memory_usage_bytes: Some(3.0),
      peer_count: 4,
    },
    recent_errors: vec![],
    configuration: DiagnosticConfig {
      debug_mode: false,
      locale: "en".to_string(),
      theme: "light".to_string(),
      log_buffer_size: 1000,
    },
  };

  let value = serde_json::to_value(&report).expect("report must serialise");
  let top_level_keys: Vec<String> = value
    .as_object()
    .expect("report must serialise as an object")
    .keys()
    .cloned()
    .collect();

  for key in &top_level_keys {
    assert!(
      ALLOWED_REPORT_KEYS.contains(&key.as_str()),
      "unexpected top-level field `{key}` — update ALLOWED_REPORT_KEYS \
       and review for privacy implications before merging"
    );
  }
  // Also ensure we didn't silently drop a field on either side.
  for expected in ALLOWED_REPORT_KEYS {
    assert!(
      top_level_keys.iter().any(|k| k == expected),
      "whitelisted field `{expected}` disappeared from DiagnosticReport"
    );
  }
}

#[test]
fn diagnostic_performance_fields_are_whitelisted() {
  let metrics = PerformanceMetrics {
    page_load_ms: Some(1.0),
    ws_latency_ms: Some(2.0),
    memory_usage_bytes: Some(3.0),
    peer_count: 4,
  };
  let value = serde_json::to_value(&metrics).expect("metrics must serialise");
  let keys: Vec<String> = value.as_object().unwrap().keys().cloned().collect();

  for key in &keys {
    assert!(
      ALLOWED_PERFORMANCE_KEYS.contains(&key.as_str()),
      "unexpected PerformanceMetrics field `{key}`"
    );
  }
  for expected in ALLOWED_PERFORMANCE_KEYS {
    assert!(
      keys.iter().any(|k| k == expected),
      "whitelisted PerformanceMetrics field `{expected}` disappeared"
    );
  }
}

#[test]
fn diagnostic_configuration_fields_are_whitelisted() {
  let config = DiagnosticConfig {
    debug_mode: false,
    locale: "en".to_string(),
    theme: "light".to_string(),
    log_buffer_size: 1000,
  };
  let value = serde_json::to_value(&config).expect("config must serialise");
  let keys: Vec<String> = value.as_object().unwrap().keys().cloned().collect();

  for key in &keys {
    assert!(
      ALLOWED_CONFIG_KEYS.contains(&key.as_str()),
      "unexpected DiagnosticConfig field `{key}`"
    );
  }
  for expected in ALLOWED_CONFIG_KEYS {
    assert!(
      keys.iter().any(|k| k == expected),
      "whitelisted DiagnosticConfig field `{expected}` disappeared"
    );
  }
}

#[test]
fn diagnostic_report_field_names_never_contain_sensitive_substrings() {
  // Belt-and-braces: even if a future PR sneaks a field past the
  // whitelist reviewer, this smoke test catches obvious leaks like
  // `jwt_token`, `user_password`, `session_secret`, etc.
  let report = DiagnosticReport {
    timestamp: String::new(),
    user_agent: String::new(),
    connected: false,
    performance: PerformanceMetrics {
      page_load_ms: None,
      ws_latency_ms: None,
      memory_usage_bytes: None,
      peer_count: 0,
    },
    recent_errors: vec![LogEntry {
      timestamp: 0,
      level: LogLevel::Error,
      module: "m".to_string(),
      message: "msg".to_string(),
      data: None,
    }],
    configuration: DiagnosticConfig {
      debug_mode: false,
      locale: String::new(),
      theme: String::new(),
      log_buffer_size: 0,
    },
  };

  let value = serde_json::to_value(&report).expect("report must serialise");
  let mut all_keys = std::collections::BTreeSet::new();
  collect_json_keys(&value, &mut all_keys);

  for key in &all_keys {
    let lower = key.to_lowercase();
    for needle in SENSITIVE_FIELD_SUBSTRINGS {
      assert!(
        !lower.contains(needle),
        "field `{key}` contains sensitive-looking substring `{needle}` — \
         diagnostic reports MUST NOT embed credentials or secrets"
      );
    }
  }
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
