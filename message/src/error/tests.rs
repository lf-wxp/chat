use super::*;

#[test]
fn test_error_code_format() {
  // Signaling codes
  assert_eq!(SIG001.to_code_string(), "SIG001");
  assert_eq!(SIG002.to_code_string(), "SIG002");
  assert_eq!(SIG003.to_code_string(), "SIG003");
  assert_eq!(SIG101.to_code_string(), "SIG101");
  assert_eq!(SIG104.to_code_string(), "SIG104");

  // Chat codes
  assert_eq!(CHT001.to_code_string(), "CHT001");
  assert_eq!(CHT101.to_code_string(), "CHT101");
  assert_eq!(CHT103.to_code_string(), "CHT103");
  assert_eq!(CHT104.to_code_string(), "CHT104");
  assert_eq!(CHT105.to_code_string(), "CHT105");

  // Audio/Video codes
  assert_eq!(AV001.to_code_string(), "AV001");
  assert_eq!(AV401.to_code_string(), "AV401");
  assert_eq!(AV405.to_code_string(), "AV405");

  // Room codes
  assert_eq!(ROM001.to_code_string(), "ROM001");
  assert_eq!(ROM101.to_code_string(), "ROM101");
  assert_eq!(ROM102.to_code_string(), "ROM102");
  assert_eq!(ROM103.to_code_string(), "ROM103");
  assert_eq!(ROM104.to_code_string(), "ROM104");
  assert_eq!(ROM105.to_code_string(), "ROM105");
  assert_eq!(ROM106.to_code_string(), "ROM106");
  assert_eq!(ROM107.to_code_string(), "ROM107");
  assert_eq!(ROM108.to_code_string(), "ROM108");

  // Theater codes
  assert_eq!(THR001.to_code_string(), "THR001");
  assert_eq!(THR101.to_code_string(), "THR101");
  assert_eq!(THR103.to_code_string(), "THR103");
  assert_eq!(THR104.to_code_string(), "THR104");

  // File codes
  assert_eq!(FIL001.to_code_string(), "FIL001");
  assert_eq!(FIL101.to_code_string(), "FIL101");

  // Auth codes
  assert_eq!(AUTH001.to_code_string(), "AUTH001");
  assert_eq!(AUTH501.to_code_string(), "AUTH501");

  // Persistence codes
  assert_eq!(PST301.to_code_string(), "PST301");

  // System codes
  assert_eq!(SYS001.to_code_string(), "SYS001");
  assert_eq!(SYS101.to_code_string(), "SYS101");
  assert_eq!(SYS301.to_code_string(), "SYS301");
}

#[test]
fn test_error_code_i18n_key() {
  assert_eq!(SIG001.to_i18n_key(), "error.sig001");
  assert_eq!(CHT101.to_i18n_key(), "error.cht101");
  assert_eq!(ROM104.to_i18n_key(), "error.rom104");
  assert_eq!(THR103.to_i18n_key(), "error.thr103");
  assert_eq!(SYS301.to_i18n_key(), "error.sys301");
}

#[test]
fn test_error_response_creation() {
  let response = ErrorResponse::new(SIG001, "WebSocket connection failed", "trace123");
  assert_eq!(response.code, SIG001);
  assert_eq!(response.message, "WebSocket connection failed");
  assert_eq!(response.i18n_key, "error.sig001");
  assert_eq!(response.trace_id, "trace123");
  assert!(response.details.is_empty());
}

#[test]
fn test_error_response_with_details() {
  let response = ErrorResponse::new(SIG003, "ICE connection failed", "trace456")
    .with_detail("retry_count", "3")
    .with_detail("ice_state", "failed");

  assert_eq!(response.details.len(), 2);
  assert_eq!(
    response.details.get("retry_count").unwrap(),
    &"3".to_string()
  );
  assert_eq!(
    response.details.get("ice_state").unwrap(),
    &"failed".to_string()
  );
}

#[test]
fn test_error_code_bitcode_roundtrip() {
  let codes = [
    SIG001, SIG101, CHT001, CHT101, CHT103, AV401, ROM104, ROM108, THR103, THR104, FIL101, AUTH501,
    PST301, SYS301,
  ];

  for code in codes {
    let encoded = bitcode::encode(&code);
    let decoded: ErrorCode = bitcode::decode(&encoded).unwrap();
    assert_eq!(code, decoded);
  }
}

#[test]
fn test_error_response_bitcode_roundtrip() {
  let response = ErrorResponse::new(SIG003, "ICE connection failed", "trace789")
    .with_detail("ice_state", "failed")
    .with_detail("retry_count", "3");

  let encoded = bitcode::encode(&response);
  let decoded: ErrorResponse = bitcode::decode(&encoded).unwrap();

  assert_eq!(response.code, decoded.code);
  assert_eq!(response.message, decoded.message);
  assert_eq!(response.i18n_key, decoded.i18n_key);
  assert_eq!(response.details, decoded.details);
  assert_eq!(response.trace_id, decoded.trace_id);
}

#[test]
fn test_error_response_json_roundtrip() {
  let response =
    ErrorResponse::new(ROM104, "User already in room", "trace123").with_detail("user_id", "abc123");

  let json = serde_json::to_string(&response).unwrap();
  let decoded: ErrorResponse = serde_json::from_str(&json).unwrap();

  assert_eq!(response.code, decoded.code);
  assert_eq!(response.message, decoded.message);
  assert_eq!(response.i18n_key, decoded.i18n_key);
  assert_eq!(response.details, decoded.details);
  assert_eq!(response.trace_id, decoded.trace_id);
}

// =============================================================================
// Comprehensive ErrorModule Tests
// =============================================================================

#[test]
fn test_all_error_module_to_code_string() {
  // Test all ErrorModule variants
  assert_eq!(
    ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 1).to_code_string(),
    "SIG001"
  );
  assert_eq!(
    ErrorCode::new(ErrorModule::Cht, ErrorCategory::Client, 1).to_code_string(),
    "CHT101"
  );
  assert_eq!(
    ErrorCode::new(ErrorModule::Av, ErrorCategory::Media, 1).to_code_string(),
    "AV401"
  );
  assert_eq!(
    ErrorCode::new(ErrorModule::Rom, ErrorCategory::Client, 1).to_code_string(),
    "ROM101"
  );
  assert_eq!(
    ErrorCode::new(ErrorModule::E2e, ErrorCategory::Security, 1).to_code_string(),
    "E2E501"
  );
  assert_eq!(
    ErrorCode::new(ErrorModule::Fil, ErrorCategory::Network, 1).to_code_string(),
    "FIL001"
  );
  assert_eq!(
    ErrorCode::new(ErrorModule::Thr, ErrorCategory::Client, 1).to_code_string(),
    "THR101"
  );
  assert_eq!(
    ErrorCode::new(ErrorModule::Auth, ErrorCategory::Security, 1).to_code_string(),
    "AUTH501"
  );
  assert_eq!(
    ErrorCode::new(ErrorModule::Pst, ErrorCategory::Server, 1).to_code_string(),
    "PST301"
  );
  assert_eq!(
    ErrorCode::new(ErrorModule::Sys, ErrorCategory::Client, 1).to_code_string(),
    "SYS101"
  );
}

#[test]
fn test_all_error_category_values() {
  // Test all ErrorCategory discriminant values
  assert_eq!(ErrorCategory::Network as u16, 0);
  assert_eq!(ErrorCategory::Client as u16, 1);
  assert_eq!(ErrorCategory::Informational as u16, 2);
  assert_eq!(ErrorCategory::Server as u16, 3);
  assert_eq!(ErrorCategory::Media as u16, 4);
  assert_eq!(ErrorCategory::Security as u16, 5);
}

#[test]
fn test_error_category_in_code_string() {
  // Verify category affects numeric portion correctly
  let base_seq = 5u16;

  // Network = 0, so numeric = 0 * 100 + 5 = 5 -> "005"
  assert_eq!(
    ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, base_seq).to_code_string(),
    "SIG005"
  );

  // Client = 1, so numeric = 1 * 100 + 5 = 105 -> "105"
  assert_eq!(
    ErrorCode::new(ErrorModule::Sig, ErrorCategory::Client, base_seq).to_code_string(),
    "SIG105"
  );

  // Informational = 2, so numeric = 2 * 100 + 5 = 205 -> "205"
  assert_eq!(
    ErrorCode::new(ErrorModule::Sig, ErrorCategory::Informational, base_seq).to_code_string(),
    "SIG205"
  );

  // Server = 3, so numeric = 3 * 100 + 5 = 305 -> "305"
  assert_eq!(
    ErrorCode::new(ErrorModule::Sig, ErrorCategory::Server, base_seq).to_code_string(),
    "SIG305"
  );

  // Media = 4, so numeric = 4 * 100 + 5 = 405 -> "405"
  assert_eq!(
    ErrorCode::new(ErrorModule::Sig, ErrorCategory::Media, base_seq).to_code_string(),
    "SIG405"
  );

  // Security = 5, so numeric = 5 * 100 + 5 = 505 -> "505"
  assert_eq!(
    ErrorCode::new(ErrorModule::Sig, ErrorCategory::Security, base_seq).to_code_string(),
    "SIG505"
  );
}

#[test]
fn test_error_code_display_trait() {
  let code = ErrorCode::new(ErrorModule::Auth, ErrorCategory::Security, 1);
  assert_eq!(format!("{code}"), "AUTH501");

  let code2 = ErrorCode::new(ErrorModule::Rom, ErrorCategory::Client, 12);
  assert_eq!(format!("{code2}"), "ROM112");
}

#[test]
fn test_error_module_serialization() {
  // Test bitcode serialization for all ErrorModule variants
  let modules = [
    ErrorModule::Sig,
    ErrorModule::Cht,
    ErrorModule::Av,
    ErrorModule::Rom,
    ErrorModule::E2e,
    ErrorModule::Fil,
    ErrorModule::Thr,
    ErrorModule::Auth,
    ErrorModule::Pst,
    ErrorModule::Sys,
  ];

  for module in modules {
    let encoded = bitcode::encode(&module);
    let decoded: ErrorModule = bitcode::decode(&encoded).unwrap();
    assert_eq!(module, decoded);
  }
}

#[test]
fn test_error_category_serialization() {
  // Test bitcode serialization for all ErrorCategory variants
  let categories = [
    ErrorCategory::Network,
    ErrorCategory::Client,
    ErrorCategory::Informational,
    ErrorCategory::Server,
    ErrorCategory::Media,
    ErrorCategory::Security,
  ];

  for category in categories {
    let encoded = bitcode::encode(&category);
    let decoded: ErrorCategory = bitcode::decode(&encoded).unwrap();
    assert_eq!(category, decoded);
  }
}

#[test]
fn test_error_response_timestamp() {
  let before = chrono::Utc::now();
  let response = ErrorResponse::new(SIG001, "test", "trace");
  let after = chrono::Utc::now();

  let timestamp = response.timestamp();
  assert!(timestamp >= before);
  assert!(timestamp <= after);
}

#[test]
fn test_multi_digit_sequence() {
  // Test that multi-digit sequences are formatted correctly
  let code = ErrorCode::new(ErrorModule::Rom, ErrorCategory::Client, 12);
  assert_eq!(code.to_code_string(), "ROM112");

  let code2 = ErrorCode::new(ErrorModule::Rom, ErrorCategory::Client, 1);
  assert_eq!(code2.to_code_string(), "ROM101");
}

#[test]
fn test_message_error_variants() {
  // Test MessageError Display implementations
  assert_eq!(
    format!("{}", MessageError::InvalidFormat),
    "invalid message format"
  );
  assert_eq!(
    format!("{}", MessageError::Serialization("test error".to_string())),
    "serialization error: test error"
  );
  assert_eq!(
    format!(
      "{}",
      MessageError::Deserialization("parse failed".to_string())
    ),
    "deserialization error: parse failed"
  );
  assert_eq!(
    format!("{}", MessageError::InvalidDiscriminator(255)),
    "invalid discriminator: 255"
  );
  assert_eq!(
    format!("{}", MessageError::Validation("too long".to_string())),
    "validation error: too long"
  );
}
