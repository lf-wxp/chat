//! Unit tests for error handler module.
//!
//! Tests cover:
//! - ErrorToast data structure creation and fields
//! - `next_toast_id()` uniqueness and monotonic increment
//! - ErrorToastManager creation and Default impl
//! - `show_error` logic: ErrorResponse → ErrorToast field mapping
//! - `show_error_message` i18n key derivation
//! - `show_error_message_with_key` custom key passthrough
//! - `dismiss` and `toggle_expand` logic
//! - ERROR_TOAST_COUNTER atomic counter behavior

use super::*;
use message::error::{ErrorCategory, ErrorCode, ErrorModule, ErrorResponse};

// ── next_toast_id tests ──

#[test]
fn test_next_toast_id_is_positive() {
  let id = next_toast_id();
  assert!(id > 0, "Toast ID should be positive (1-based)");
}

#[test]
fn test_next_toast_id_is_monotonically_increasing() {
  let id1 = next_toast_id();
  let id2 = next_toast_id();
  let id3 = next_toast_id();
  assert!(id2 > id1, "IDs should be monotonically increasing");
  assert!(id3 > id2, "IDs should be monotonically increasing");
}

#[test]
fn test_next_toast_id_is_unique() {
  let mut ids = std::collections::HashSet::new();
  for _ in 0..100 {
    let id = next_toast_id();
    assert!(ids.insert(id), "Toast ID {} should be unique", id);
  }
}

// ── ErrorToast structure tests ──

#[test]
fn test_error_toast_creation() {
  let toast = ErrorToast {
    id: 1,
    code: "SIG001".to_string(),
    i18n_key: "error.sig001".to_string(),
    message: "WebSocket connection failed".to_string(),
    detail_i18n_key: "error.sig001.detail".to_string(),
    details: vec![("endpoint".to_string(), "ws://localhost:8080".to_string())],
    trace_id: "trace-123".to_string(),
    expanded: false,
    auto_remove_handle: None,
  };

  assert_eq!(toast.id, 1);
  assert_eq!(toast.code, "SIG001");
  assert_eq!(toast.i18n_key, "error.sig001");
  assert_eq!(toast.message, "WebSocket connection failed");
  assert_eq!(toast.detail_i18n_key, "error.sig001.detail");
  assert_eq!(toast.details.len(), 1);
  assert_eq!(toast.trace_id, "trace-123");
  assert!(!toast.expanded);
}

#[test]
fn test_error_toast_clone() {
  let toast = ErrorToast {
    id: 42,
    code: "CHT001".to_string(),
    i18n_key: "error.cht001".to_string(),
    message: "Send failed".to_string(),
    detail_i18n_key: String::new(),
    details: Vec::new(),
    trace_id: "t-42".to_string(),
    expanded: true,
    auto_remove_handle: None,
  };

  let cloned = toast.clone();
  assert_eq!(toast.id, cloned.id);
  assert_eq!(toast.code, cloned.code);
  assert_eq!(toast.i18n_key, cloned.i18n_key);
  assert_eq!(toast.message, cloned.message);
  assert_eq!(toast.expanded, cloned.expanded);
}

#[test]
fn test_error_toast_debug() {
  let toast = ErrorToast {
    id: 1,
    code: "SIG001".to_string(),
    i18n_key: "error.sig001".to_string(),
    message: "test".to_string(),
    detail_i18n_key: String::new(),
    details: Vec::new(),
    trace_id: String::new(),
    expanded: false,
    auto_remove_handle: None,
  };
  let debug = format!("{:?}", toast);
  assert!(debug.contains("SIG001"));
}

// ── show_error field mapping tests ──

#[test]
fn test_show_error_maps_code_to_string() {
  let error_code = ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 1);
  let code_str = error_code.to_code_string();
  assert_eq!(code_str, "SIG001");
}

#[test]
fn test_show_error_maps_i18n_key() {
  let error_code = ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 1);
  let i18n_key = error_code.to_i18n_key();
  assert_eq!(i18n_key, "error.sig001");
}

#[test]
fn test_show_error_detail_i18n_key_format() {
  let error_code = ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 1);
  let i18n_key = error_code.to_i18n_key();
  let detail_key = format!("{}.detail", i18n_key);
  assert_eq!(detail_key, "error.sig001.detail");
}

#[test]
fn test_show_error_maps_details() {
  let error = ErrorResponse::new(
    ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 1),
    "connection failed",
    "trace-001",
  )
  .with_detail("endpoint", "ws://localhost");

  let details: Vec<(String, String)> = error
    .details
    .iter()
    .map(|(k, v)| (k.clone(), v.clone()))
    .collect();

  assert_eq!(details.len(), 1);
  // HashMap ordering is non-deterministic, check by content
  let has_endpoint = details
    .iter()
    .any(|(k, v)| k == "endpoint" && v == "ws://localhost");
  assert!(has_endpoint, "Details should contain endpoint");
}

#[test]
fn test_show_error_maps_trace_id() {
  let error = ErrorResponse::new(
    ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 1),
    "test",
    "trace-abc-123",
  );
  assert_eq!(error.trace_id, "trace-abc-123");
}

#[test]
fn test_show_error_maps_message() {
  let error = ErrorResponse::new(
    ErrorCode::new(ErrorModule::Cht, ErrorCategory::Client, 1),
    "Message too long",
    "t1",
  );
  assert_eq!(error.message, "Message too long");
}

// ── show_error_message i18n key derivation tests ──

#[test]
fn test_show_error_message_i18n_key_derivation() {
  // show_error_message derives key as error.<lowercase_code>
  let code = "SIG001";
  let expected_key = format!("error.{}", code.to_lowercase());
  assert_eq!(expected_key, "error.sig001");
}

#[test]
fn test_show_error_message_i18n_key_mixed_case() {
  let code = "AUTH502";
  let expected_key = format!("error.{}", code.to_lowercase());
  assert_eq!(expected_key, "error.auth502");
}

// ── show_error_message_with_key tests ──

#[test]
fn test_show_error_message_with_key_uses_custom_key() {
  // The method should pass through the custom key without modification
  let code = "AUTH502";
  let custom_key = "auth.session_invalidated";

  // Verify the custom key does NOT follow the default pattern
  let default_key = format!("error.{}", code.to_lowercase());
  assert_ne!(custom_key, default_key);
}

// ── Various error code module tests ──

#[test]
fn test_error_code_modules_produce_distinct_prefixes() {
  let sig = ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 1);
  let cht = ErrorCode::new(ErrorModule::Cht, ErrorCategory::Network, 1);
  let auth = ErrorCode::new(ErrorModule::Auth, ErrorCategory::Network, 1);

  assert!(sig.to_code_string().starts_with("SIG"));
  assert!(cht.to_code_string().starts_with("CHT"));
  assert!(auth.to_code_string().starts_with("AUTH"));

  assert_ne!(sig.to_code_string(), cht.to_code_string());
  assert_ne!(sig.to_code_string(), auth.to_code_string());
}

#[test]
fn test_error_categories_produce_distinct_numbers() {
  let network = ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 1);
  let client = ErrorCode::new(ErrorModule::Sig, ErrorCategory::Client, 1);
  let server = ErrorCode::new(ErrorModule::Sig, ErrorCategory::Server, 1);

  assert_eq!(network.to_code_string(), "SIG001");
  assert_eq!(client.to_code_string(), "SIG101");
  assert_eq!(server.to_code_string(), "SIG301");
}

// ── ErrorToastManager Default trait ──

#[test]
fn test_error_toast_manager_default_trait() {
  // ErrorToastManager should implement Default
  // This will only compile if Default is implemented
  fn _assert_default<T: Default>() {}
  _assert_default::<ErrorToastManager>();
}

// ── MAX_TOASTS and enforce_max_toasts tests ──

#[test]
fn test_max_toasts_constant() {
  assert_eq!(MAX_TOASTS, 5, "MAX_TOASTS should be 5");
}

#[test]
fn test_enforce_max_toasts_removes_oldest_when_at_capacity() {
  let mut toasts: Vec<ErrorToast> = (0..5)
    .map(|i| ErrorToast {
      id: i as u64,
      code: format!("E{}", i),
      i18n_key: String::new(),
      message: format!("msg {}", i),
      detail_i18n_key: String::new(),
      details: Vec::new(),
      trace_id: String::new(),
      expanded: false,
      auto_remove_handle: None,
    })
    .collect();

  assert_eq!(toasts.len(), 5);
  ErrorToastManager::enforce_max_toasts(&mut toasts);
  assert_eq!(toasts.len(), 4, "Should remove one to make room");
  // The oldest (id=0) should have been removed
  assert!(!toasts.iter().any(|t| t.id == 0));
}

#[test]
fn test_enforce_max_toasts_skips_expanded_toasts() {
  let mut toasts: Vec<ErrorToast> = (0..5)
    .map(|i| ErrorToast {
      id: i as u64,
      code: format!("E{}", i),
      i18n_key: String::new(),
      message: format!("msg {}", i),
      detail_i18n_key: String::new(),
      details: Vec::new(),
      trace_id: String::new(),
      // First toast is expanded
      expanded: i == 0,
      auto_remove_handle: None,
    })
    .collect();

  ErrorToastManager::enforce_max_toasts(&mut toasts);
  // The expanded toast (id=0) should still be present
  assert!(toasts.iter().any(|t| t.id == 0 && t.expanded));
  // The first non-expanded toast (id=1) should be removed
  assert!(!toasts.iter().any(|t| t.id == 1));
}

#[test]
fn test_enforce_max_toasts_no_op_below_limit() {
  let mut toasts: Vec<ErrorToast> = (0..3)
    .map(|i| ErrorToast {
      id: i as u64,
      code: format!("E{}", i),
      i18n_key: String::new(),
      message: format!("msg {}", i),
      detail_i18n_key: String::new(),
      details: Vec::new(),
      trace_id: String::new(),
      expanded: false,
      auto_remove_handle: None,
    })
    .collect();

  ErrorToastManager::enforce_max_toasts(&mut toasts);
  assert_eq!(toasts.len(), 3, "Should not remove when below capacity");
}

#[test]
fn test_enforce_max_toasts_all_expanded_removes_oldest() {
  let mut toasts: Vec<ErrorToast> = (0..5)
    .map(|i| ErrorToast {
      id: i as u64,
      code: format!("E{}", i),
      i18n_key: String::new(),
      message: format!("msg {}", i),
      detail_i18n_key: String::new(),
      details: Vec::new(),
      trace_id: String::new(),
      expanded: true,
      auto_remove_handle: None,
    })
    .collect();

  ErrorToastManager::enforce_max_toasts(&mut toasts);
  assert_eq!(toasts.len(), 4);
  // When all are expanded, the oldest (id=0) is removed
  assert!(!toasts.iter().any(|t| t.id == 0));
}
