//! Error code system for WebRTC Chat Application.
//!
//! This module provides a unified error code system with i18n key mapping
//! and input validation utilities.

use bitcode::{Decode, Encode};
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

// ============================================================================
// Error Module and Category Definitions
// ============================================================================

/// Error code prefix for each module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode)]
pub enum ErrorModule {
  /// Signaling (connection, SDP/ICE exchange, heartbeat)
  Sig,
  /// Chat (message send/receive, sticker, voice, image)
  Cht,
  /// Audio/Video (calls, screen share, media devices)
  Av,
  /// Room (create, join, leave, permissions)
  Rom,
  /// End-to-End Encryption (key exchange, encrypt/decrypt)
  E2e,
  /// File Transfer (upload, download, chunk, resume)
  Fil,
  /// Theater (video share, danmaku, owner controls)
  Thr,
  /// Authentication/Session (login, JWT, recovery)
  Auth,
  /// Persistence (`IndexedDB`, message storage)
  Pst,
  /// System (general errors, browser compatibility)
  Sys,
}

/// Error category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode)]
pub enum ErrorCategory {
  /// Network error (connection lost, timeout)
  Network = 0,
  /// Client error (invalid input, permission denied)
  Client = 1,
  /// Informational (not an error, status update)
  Informational = 2,
  /// Server error (internal failure)
  Server = 3,
  /// Media error (device access, codec issue)
  Media = 4,
  /// Security error (encryption, authentication)
  Security = 5,
}

// ============================================================================
// Error Code Definition
// ============================================================================

/// Unified error code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode)]
pub struct ErrorCode {
  /// Module prefix
  pub module: ErrorModule,
  /// Error category
  pub category: ErrorCategory,
  /// Sequence number within module-category
  pub sequence: u8,
}

impl ErrorCode {
  /// Create a new error code.
  #[must_use]
  pub const fn new(module: ErrorModule, category: ErrorCategory, sequence: u8) -> Self {
    Self {
      module,
      category,
      sequence,
    }
  }

  /// Convert to string format (e.g., "SIG001", "SIG101").
  #[must_use]
  pub fn to_code_string(&self) -> String {
    let module_str = match self.module {
      ErrorModule::Sig => "SIG",
      ErrorModule::Cht => "CHT",
      ErrorModule::Av => "AV",
      ErrorModule::Rom => "ROM",
      ErrorModule::E2e => "E2E",
      ErrorModule::Fil => "FIL",
      ErrorModule::Thr => "THR",
      ErrorModule::Auth => "AUTH",
      ErrorModule::Pst => "PST",
      ErrorModule::Sys => "SYS",
    };
    format!(
      "{}{:01}{:02}",
      module_str, self.category as u8, self.sequence
    )
  }

  /// Get the i18n key for this error code.
  #[must_use]
  pub fn to_i18n_key(&self) -> String {
    let code = self.to_code_string();
    format!("error.{}", code.to_lowercase())
  }
}

impl std::fmt::Display for ErrorCode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.to_code_string())
  }
}

// ============================================================================
// Error Response Structure
// ============================================================================

/// Error response structure.
///
/// Uses timestamp-based encoding for `DateTime` field to work with bitcode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct ErrorResponse {
  /// Error code
  pub code: ErrorCode,
  /// Default English error message
  pub message: String,
  /// i18n key for localized message
  pub i18n_key: String,
  /// Optional contextual details (key-value pairs)
  #[serde(default, skip_serializing_if = "HashMap::is_empty")]
  pub details: HashMap<String, String>,
  /// Timestamp when error occurred (Unix timestamp in nanoseconds)
  #[serde(
    serialize_with = "serialize_timestamp_nanos",
    deserialize_with = "deserialize_timestamp_nanos"
  )]
  pub timestamp_nanos: i64,
  /// Unique trace ID for logging
  pub trace_id: String,
}

impl ErrorResponse {
  /// Create a new error response.
  #[must_use]
  pub fn new(code: ErrorCode, message: impl Into<String>, trace_id: impl Into<String>) -> Self {
    let message = message.into();
    let i18n_key = code.to_i18n_key();
    Self {
      code,
      message,
      i18n_key,
      details: HashMap::new(),
      timestamp_nanos: Utc::now().timestamp_nanos_opt().unwrap_or(0),
      trace_id: trace_id.into(),
    }
  }

  /// Add contextual details to the error.
  #[must_use]
  pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
    self.details.insert(key.into(), value.into());
    self
  }

  /// Get the timestamp as `DateTime<Utc>`.
  #[must_use]
  pub fn timestamp(&self) -> DateTime<Utc> {
    Utc.timestamp_nanos(self.timestamp_nanos)
  }
}

/// Serialize timestamp nanos to ISO 8601 string.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn serialize_timestamp_nanos<S>(nanos: &i64, serializer: S) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
  let dt = Utc.timestamp_nanos(*nanos);
  serializer.serialize_str(&dt.to_rfc3339())
}

/// Deserialize ISO 8601 string to timestamp nanos.
fn deserialize_timestamp_nanos<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
  D: serde::Deserializer<'de>,
{
  let s = String::deserialize(deserializer)?;
  let dt = DateTime::parse_from_rfc3339(&s).map_err(serde::de::Error::custom)?;
  Ok(dt.timestamp_nanos_opt().unwrap_or(0))
}

// ============================================================================
// Message Error Type
// ============================================================================

/// Message error type.
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub enum MessageError {
  /// Invalid message format
  #[error("invalid message format")]
  InvalidFormat,
  /// Serialization error
  #[error("serialization error: {0}")]
  Serialization(String),
  /// Deserialization error
  #[error("deserialization error: {0}")]
  Deserialization(String),
  /// Invalid discriminator
  #[error("invalid discriminator: {0}")]
  InvalidDiscriminator(u8),
  /// Validation error
  #[error("validation error: {0}")]
  Validation(String),
}

impl From<bitcode::Error> for MessageError {
  fn from(err: bitcode::Error) -> Self {
    Self::Serialization(err.to_string())
  }
}

// ============================================================================
// Error Code Constants - Signaling (SIG)
// ============================================================================

/// WebSocket connection failed
pub const SIG001: ErrorCode = ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 1);
/// SDP negotiation timeout
pub const SIG002: ErrorCode = ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 2);
/// ICE connection failed
pub const SIG003: ErrorCode = ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 3);
/// Heartbeat timeout
pub const SIG004: ErrorCode = ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 4);
/// WebSocket reconnection failed
pub const SIG005: ErrorCode = ErrorCode::new(ErrorModule::Sig, ErrorCategory::Network, 5);
/// Invalid SDP format
pub const SIG101: ErrorCode = ErrorCode::new(ErrorModule::Sig, ErrorCategory::Client, 1);
/// Invalid ICE candidate
pub const SIG102: ErrorCode = ErrorCode::new(ErrorModule::Sig, ErrorCategory::Client, 2);
/// Invalid message type
pub const SIG103: ErrorCode = ErrorCode::new(ErrorModule::Sig, ErrorCategory::Client, 3);
/// Rate limit exceeded
pub const SIG104: ErrorCode = ErrorCode::new(ErrorModule::Sig, ErrorCategory::Client, 4);

// ============================================================================
// Error Code Constants - Chat (CHT)
// ============================================================================

/// `DataChannel` send failed
pub const CHT001: ErrorCode = ErrorCode::new(ErrorModule::Cht, ErrorCategory::Network, 1);
/// `DataChannel` receive failed
pub const CHT002: ErrorCode = ErrorCode::new(ErrorModule::Cht, ErrorCategory::Network, 2);
/// Message ACK timeout
pub const CHT003: ErrorCode = ErrorCode::new(ErrorModule::Cht, ErrorCategory::Network, 3);
/// Message too long (max 10000 chars)
pub const CHT101: ErrorCode = ErrorCode::new(ErrorModule::Cht, ErrorCategory::Client, 1);
/// Invalid sticker ID
pub const CHT102: ErrorCode = ErrorCode::new(ErrorModule::Cht, ErrorCategory::Client, 2);
/// Message revoke timeout (2 minutes exceeded)
pub const CHT103: ErrorCode = ErrorCode::new(ErrorModule::Cht, ErrorCategory::Client, 3);
/// Message already revoked
pub const CHT104: ErrorCode = ErrorCode::new(ErrorModule::Cht, ErrorCategory::Client, 4);
/// Empty message not allowed
pub const CHT105: ErrorCode = ErrorCode::new(ErrorModule::Cht, ErrorCategory::Client, 5);
/// Encryption failed
pub const CHT501: ErrorCode = ErrorCode::new(ErrorModule::Cht, ErrorCategory::Security, 1);
/// Decryption failed
pub const CHT502: ErrorCode = ErrorCode::new(ErrorModule::Cht, ErrorCategory::Security, 2);

// ============================================================================
// Error Code Constants - Audio/Video (AV)
// ============================================================================

/// `PeerConnection` disconnected during call
pub const AV001: ErrorCode = ErrorCode::new(ErrorModule::Av, ErrorCategory::Network, 1);
/// Media connection timeout
pub const AV002: ErrorCode = ErrorCode::new(ErrorModule::Av, ErrorCategory::Network, 2);
/// Camera access denied
pub const AV401: ErrorCode = ErrorCode::new(ErrorModule::Av, ErrorCategory::Media, 1);
/// Microphone access denied
pub const AV402: ErrorCode = ErrorCode::new(ErrorModule::Av, ErrorCategory::Media, 2);
/// Screen share cancelled
pub const AV403: ErrorCode = ErrorCode::new(ErrorModule::Av, ErrorCategory::Media, 3);
/// Screen share denied
pub const AV404: ErrorCode = ErrorCode::new(ErrorModule::Av, ErrorCategory::Media, 4);
/// Codec not supported
pub const AV405: ErrorCode = ErrorCode::new(ErrorModule::Av, ErrorCategory::Media, 5);

// ============================================================================
// Error Code Constants - Room (ROM)
// ============================================================================

/// Room join timeout
pub const ROM001: ErrorCode = ErrorCode::new(ErrorModule::Rom, ErrorCategory::Network, 1);
/// Room leave timeout
pub const ROM002: ErrorCode = ErrorCode::new(ErrorModule::Rom, ErrorCategory::Network, 2);
/// Room password incorrect
pub const ROM101: ErrorCode = ErrorCode::new(ErrorModule::Rom, ErrorCategory::Client, 1);
/// Room is full (max 8 members)
pub const ROM102: ErrorCode = ErrorCode::new(ErrorModule::Rom, ErrorCategory::Client, 2);
/// Insufficient permissions
pub const ROM103: ErrorCode = ErrorCode::new(ErrorModule::Rom, ErrorCategory::Client, 3);
/// User already in room
pub const ROM104: ErrorCode = ErrorCode::new(ErrorModule::Rom, ErrorCategory::Client, 4);
/// Room not found
pub const ROM105: ErrorCode = ErrorCode::new(ErrorModule::Rom, ErrorCategory::Client, 5);
/// User banned from room
pub const ROM106: ErrorCode = ErrorCode::new(ErrorModule::Rom, ErrorCategory::Client, 6);
/// User muted in room
pub const ROM107: ErrorCode = ErrorCode::new(ErrorModule::Rom, ErrorCategory::Client, 7);
/// Cannot kick/modify owner
pub const ROM108: ErrorCode = ErrorCode::new(ErrorModule::Rom, ErrorCategory::Client, 8);

// ============================================================================
// Error Code Constants - Theater (THR)
// ============================================================================

/// Theater video stream failed
pub const THR001: ErrorCode = ErrorCode::new(ErrorModule::Thr, ErrorCategory::Network, 1);
/// Theater sync timeout
pub const THR002: ErrorCode = ErrorCode::new(ErrorModule::Thr, ErrorCategory::Network, 2);
/// Theater owner disconnected
pub const THR003: ErrorCode = ErrorCode::new(ErrorModule::Thr, ErrorCategory::Network, 3);
/// Not theater owner
pub const THR101: ErrorCode = ErrorCode::new(ErrorModule::Thr, ErrorCategory::Client, 1);
/// Invalid video source
pub const THR102: ErrorCode = ErrorCode::new(ErrorModule::Thr, ErrorCategory::Client, 2);
/// Danmaku too long (max 100 chars)
pub const THR103: ErrorCode = ErrorCode::new(ErrorModule::Thr, ErrorCategory::Client, 3);
/// Subtitle format invalid
pub const THR104: ErrorCode = ErrorCode::new(ErrorModule::Thr, ErrorCategory::Client, 4);

// ============================================================================
// Error Code Constants - File Transfer (FIL)
// ============================================================================

/// File transfer interrupted
pub const FIL001: ErrorCode = ErrorCode::new(ErrorModule::Fil, ErrorCategory::Network, 1);
/// File chunk timeout
pub const FIL002: ErrorCode = ErrorCode::new(ErrorModule::Fil, ErrorCategory::Network, 2);
/// File too large (single: 100MB, multi: 20MB)
pub const FIL101: ErrorCode = ErrorCode::new(ErrorModule::Fil, ErrorCategory::Client, 1);
/// File type not allowed
pub const FIL102: ErrorCode = ErrorCode::new(ErrorModule::Fil, ErrorCategory::Client, 2);
/// File hash mismatch
pub const FIL103: ErrorCode = ErrorCode::new(ErrorModule::Fil, ErrorCategory::Client, 3);
/// Dangerous file extension warning
pub const FIL104: ErrorCode = ErrorCode::new(ErrorModule::Fil, ErrorCategory::Client, 4);

// ============================================================================
// Error Code Constants - Authentication (AUTH)
// ============================================================================

/// Authentication timeout
pub const AUTH001: ErrorCode = ErrorCode::new(ErrorModule::Auth, ErrorCategory::Network, 1);
/// JWT token expired
pub const AUTH501: ErrorCode = ErrorCode::new(ErrorModule::Auth, ErrorCategory::Security, 1);
/// JWT token invalid
pub const AUTH502: ErrorCode = ErrorCode::new(ErrorModule::Auth, ErrorCategory::Security, 2);
/// Session invalidated (another device logged in)
pub const AUTH503: ErrorCode = ErrorCode::new(ErrorModule::Auth, ErrorCategory::Security, 3);
/// Invalid credentials
pub const AUTH101: ErrorCode = ErrorCode::new(ErrorModule::Auth, ErrorCategory::Client, 1);
/// User already exists
pub const AUTH102: ErrorCode = ErrorCode::new(ErrorModule::Auth, ErrorCategory::Client, 2);
/// User not found
pub const AUTH103: ErrorCode = ErrorCode::new(ErrorModule::Auth, ErrorCategory::Client, 3);

// ============================================================================
// Error Code Constants - Persistence (PST)
// ============================================================================

/// `IndexedDB` write failed
pub const PST301: ErrorCode = ErrorCode::new(ErrorModule::Pst, ErrorCategory::Server, 1);
/// `IndexedDB` read failed
pub const PST302: ErrorCode = ErrorCode::new(ErrorModule::Pst, ErrorCategory::Server, 2);
/// Storage quota exceeded
pub const PST303: ErrorCode = ErrorCode::new(ErrorModule::Pst, ErrorCategory::Server, 3);

// ============================================================================
// Error Code Constants - System (SYS)
// ============================================================================

/// Browser offline
pub const SYS001: ErrorCode = ErrorCode::new(ErrorModule::Sys, ErrorCategory::Network, 1);
/// WebRTC not supported
pub const SYS101: ErrorCode = ErrorCode::new(ErrorModule::Sys, ErrorCategory::Client, 1);
/// WebSocket not supported
pub const SYS102: ErrorCode = ErrorCode::new(ErrorModule::Sys, ErrorCategory::Client, 2);
/// `IndexedDB` not supported
pub const SYS103: ErrorCode = ErrorCode::new(ErrorModule::Sys, ErrorCategory::Client, 3);
/// Browser not supported
pub const SYS301: ErrorCode = ErrorCode::new(ErrorModule::Sys, ErrorCategory::Server, 1);

// ============================================================================
// Input Validation Utilities
// ============================================================================

/// Maximum lengths for various input fields
pub mod max_lengths {
  /// Maximum username length
  pub const USERNAME: usize = 20;
  /// Maximum nickname length
  pub const NICKNAME: usize = 20;
  /// Maximum room name length
  pub const ROOM_NAME: usize = 100;
  /// Maximum room description length
  pub const ROOM_DESCRIPTION: usize = 500;
  /// Maximum announcement length
  pub const ANNOUNCEMENT: usize = 500;
  /// Maximum danmaku length
  pub const DANMAKU: usize = 100;
  /// Maximum message length
  pub const MESSAGE: usize = 10000;
  /// Maximum room password length
  pub const ROOM_PASSWORD: usize = 64;
}

/// Validation result type.
pub type ValidationResult = Result<(), ValidationError>;

/// Validation error with field name and error code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
  /// The field that failed validation
  pub field: String,
  /// The error code
  pub code: ErrorCode,
  /// The error message
  pub message: String,
}

impl ValidationError {
  /// Create a new validation error.
  #[must_use]
  pub fn new(field: impl Into<String>, code: ErrorCode, message: impl Into<String>) -> Self {
    Self {
      field: field.into(),
      code,
      message: message.into(),
    }
  }
}

impl std::fmt::Display for ValidationError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}: {}", self.field, self.message)
  }
}

impl std::error::Error for ValidationError {}

/// Validate username.
///
/// Rules:
/// - Only alphanumeric characters and underscores
/// - Maximum 20 characters
/// - Minimum 3 characters
/// - Cannot start with a number
pub fn validate_username(username: &str) -> ValidationResult {
  let len = username.len();

  if len < 3 {
    return Err(ValidationError::new(
      "username",
      AUTH101,
      "Username must be at least 3 characters",
    ));
  }

  if len > max_lengths::USERNAME {
    return Err(ValidationError::new(
      "username",
      AUTH101,
      format!(
        "Username must not exceed {} characters",
        max_lengths::USERNAME
      ),
    ));
  }

  // Cannot start with a number
  if username.chars().next().is_some_and(|c| c.is_ascii_digit()) {
    return Err(ValidationError::new(
      "username",
      AUTH101,
      "Username cannot start with a number",
    ));
  }

  // Only alphanumeric and underscore
  if !username
    .chars()
    .all(|c| c.is_ascii_alphanumeric() || c == '_')
  {
    return Err(ValidationError::new(
      "username",
      AUTH101,
      "Username can only contain letters, numbers, and underscores",
    ));
  }

  Ok(())
}

/// Validate nickname.
///
/// Rules:
/// - Chinese/English characters, numbers, underscores, and spaces
/// - Maximum 20 characters
/// - Minimum 1 character
/// - Cannot be only whitespace
pub fn validate_nickname(nickname: &str) -> ValidationResult {
  let trimmed = nickname.trim();
  let len = nickname.chars().count();

  if trimmed.is_empty() {
    return Err(ValidationError::new(
      "nickname",
      AUTH101,
      "Nickname cannot be empty or only whitespace",
    ));
  }

  if len > max_lengths::NICKNAME {
    return Err(ValidationError::new(
      "nickname",
      AUTH101,
      format!(
        "Nickname must not exceed {} characters",
        max_lengths::NICKNAME
      ),
    ));
  }

  // Chinese/English, numbers, underscores, and spaces
  if !nickname.chars().all(|c| {
    c.is_ascii_alphanumeric()
      || c == '_'
      || c == ' '
      || ('\u{4E00}'..='\u{9FFF}').contains(&c)
      || ('\u{3400}'..='\u{4DBF}').contains(&c) // CJK Extension A
      || ('\u{20000}'..='\u{2A6DF}').contains(&c) // CJK Extension B
  }) {
    return Err(ValidationError::new(
      "nickname",
      AUTH101,
      "Nickname can only contain Chinese/English characters, numbers, underscores, and spaces",
    ));
  }

  Ok(())
}

/// Validate room name.
///
/// Rules:
/// - Maximum 100 characters
/// - Minimum 1 character
/// - Cannot be only whitespace
pub fn validate_room_name(name: &str) -> ValidationResult {
  let trimmed = name.trim();
  let len = name.chars().count();

  if trimmed.is_empty() {
    return Err(ValidationError::new(
      "room_name",
      ROM101,
      "Room name cannot be empty or only whitespace",
    ));
  }

  if len > max_lengths::ROOM_NAME {
    return Err(ValidationError::new(
      "room_name",
      ROM101,
      format!(
        "Room name must not exceed {} characters",
        max_lengths::ROOM_NAME
      ),
    ));
  }

  Ok(())
}

/// Validate room description.
///
/// Rules:
/// - Maximum 500 characters
pub fn validate_room_description(description: &str) -> ValidationResult {
  let len = description.chars().count();

  if len > max_lengths::ROOM_DESCRIPTION {
    return Err(ValidationError::new(
      "room_description",
      ROM101,
      format!(
        "Room description must not exceed {} characters",
        max_lengths::ROOM_DESCRIPTION
      ),
    ));
  }

  Ok(())
}

/// Validate room password.
///
/// Rules:
/// - Maximum 64 characters
/// - Minimum 1 character if provided (empty = no password)
pub fn validate_room_password(password: &str) -> ValidationResult {
  if password.is_empty() {
    return Ok(()); // Empty password is valid (no password)
  }

  let len = password.len();

  if len > max_lengths::ROOM_PASSWORD {
    return Err(ValidationError::new(
      "room_password",
      ROM101,
      format!(
        "Room password must not exceed {} characters",
        max_lengths::ROOM_PASSWORD
      ),
    ));
  }

  Ok(())
}

/// Validate announcement.
///
/// Rules:
/// - Maximum 500 characters
/// - Cannot be only whitespace
pub fn validate_announcement(announcement: &str) -> ValidationResult {
  let trimmed = announcement.trim();
  let len = announcement.chars().count();

  if trimmed.is_empty() {
    return Err(ValidationError::new(
      "announcement",
      ROM101,
      "Announcement cannot be empty or only whitespace",
    ));
  }

  if len > max_lengths::ANNOUNCEMENT {
    return Err(ValidationError::new(
      "announcement",
      ROM101,
      format!(
        "Announcement must not exceed {} characters",
        max_lengths::ANNOUNCEMENT
      ),
    ));
  }

  Ok(())
}

/// Validate danmaku content.
///
/// Rules:
/// - Maximum 100 characters
/// - Minimum 1 character
/// - Cannot be only whitespace
pub fn validate_danmaku(content: &str) -> ValidationResult {
  let trimmed = content.trim();
  let len = content.chars().count();

  if trimmed.is_empty() {
    return Err(ValidationError::new(
      "danmaku",
      THR103,
      "Danmaku cannot be empty or only whitespace",
    ));
  }

  if len > max_lengths::DANMAKU {
    return Err(ValidationError::new(
      "danmaku",
      THR103,
      format!(
        "Danmaku must not exceed {} characters",
        max_lengths::DANMAKU
      ),
    ));
  }

  Ok(())
}

/// Validate message content.
///
/// Rules:
/// - Maximum 10000 characters
/// - Minimum 1 character
/// - Cannot be only whitespace
pub fn validate_message(content: &str) -> ValidationResult {
  let trimmed = content.trim();
  let len = content.chars().count();

  if trimmed.is_empty() {
    return Err(ValidationError::new(
      "message",
      CHT105,
      "Message cannot be empty or only whitespace",
    ));
  }

  if len > max_lengths::MESSAGE {
    return Err(ValidationError::new(
      "message",
      CHT101,
      format!(
        "Message must not exceed {} characters",
        max_lengths::MESSAGE
      ),
    ));
  }

  Ok(())
}

/// Validate user ID format.
///
/// Rules:
/// - Must be a valid UUID format
pub fn validate_user_id(user_id: &str) -> ValidationResult {
  if uuid::Uuid::parse_str(user_id).is_err() {
    return Err(ValidationError::new(
      "user_id",
      AUTH101,
      "Invalid user ID format",
    ));
  }

  Ok(())
}

/// Validate room ID format.
///
/// Rules:
/// - Must be a valid UUID format
pub fn validate_room_id(room_id: &str) -> ValidationResult {
  if uuid::Uuid::parse_str(room_id).is_err() {
    return Err(ValidationError::new(
      "room_id",
      ROM105,
      "Invalid room ID format",
    ));
  }

  Ok(())
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
  use super::*;

  // ==========================================================================
  // Error Code Tests
  // ==========================================================================

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

  // ==========================================================================
  // Username Validation Tests
  // ==========================================================================

  #[test]
  fn test_validate_username_valid() {
    assert!(validate_username("alice").is_ok());
    assert!(validate_username("bob_123").is_ok());
    assert!(validate_username("user_name").is_ok());
    assert!(validate_username("abc").is_ok());
    assert!(validate_username("UserName2024").is_ok());
  }

  #[test]
  fn test_validate_username_too_short() {
    assert!(validate_username("ab").is_err());
    assert!(validate_username("a").is_err());
    assert!(validate_username("").is_err());
  }

  #[test]
  fn test_validate_username_too_long() {
    let long_name = "a".repeat(21);
    assert!(validate_username(&long_name).is_err());

    let max_name = "a".repeat(20);
    assert!(validate_username(&max_name).is_ok());
  }

  #[test]
  fn test_validate_username_starts_with_number() {
    assert!(validate_username("1user").is_err());
    assert!(validate_username("123abc").is_err());
    assert!(validate_username("0_alice").is_err());
  }

  #[test]
  fn test_validate_username_invalid_characters() {
    assert!(validate_username("user-name").is_err());
    assert!(validate_username("user.name").is_err());
    assert!(validate_username("user@name").is_err());
    assert!(validate_username("user name").is_err());
    assert!(validate_username("用户名").is_err());
  }

  // ==========================================================================
  // Nickname Validation Tests
  // ==========================================================================

  #[test]
  fn test_validate_nickname_valid() {
    assert!(validate_nickname("Alice").is_ok());
    assert!(validate_nickname("用户名").is_ok());
    assert!(validate_nickname("Bob 123").is_ok());
    assert!(validate_nickname("小明同学").is_ok());
    assert!(validate_nickname("用户_123").is_ok());
    assert!(validate_nickname("Test User").is_ok());
  }

  #[test]
  fn test_validate_nickname_empty() {
    assert!(validate_nickname("").is_err());
    assert!(validate_nickname("   ").is_err());
    assert!(validate_nickname("\t\n").is_err());
  }

  #[test]
  fn test_validate_nickname_too_long() {
    let long_nick = "a".repeat(21);
    assert!(validate_nickname(&long_nick).is_err());

    // Chinese characters count as 1 each
    let long_chinese = "测".repeat(21);
    assert!(validate_nickname(&long_chinese).is_err());

    let max_nick = "a".repeat(20);
    assert!(validate_nickname(&max_nick).is_ok());
  }

  #[test]
  fn test_validate_nickname_invalid_characters() {
    assert!(validate_nickname("user@name").is_err());
    assert!(validate_nickname("user!name").is_err());
    assert!(validate_nickname("user#name").is_err());
    assert!(validate_nickname("user$name").is_err());
  }

  // ==========================================================================
  // Room Name Validation Tests
  // ==========================================================================

  #[test]
  fn test_validate_room_name_valid() {
    assert!(validate_room_name("My Room").is_ok());
    assert!(validate_room_name("聊天室").is_ok());
    assert!(validate_room_name("Room-2024").is_ok());
    assert!(validate_room_name("a").is_ok());
  }

  #[test]
  fn test_validate_room_name_empty() {
    assert!(validate_room_name("").is_err());
    assert!(validate_room_name("   ").is_err());
  }

  #[test]
  fn test_validate_room_name_too_long() {
    let long_name = "a".repeat(101);
    assert!(validate_room_name(&long_name).is_err());

    let max_name = "a".repeat(100);
    assert!(validate_room_name(&max_name).is_ok());
  }

  // ==========================================================================
  // Room Description Validation Tests
  // ==========================================================================

  #[test]
  fn test_validate_room_description_valid() {
    assert!(validate_room_description("").is_ok()); // Empty is OK
    assert!(validate_room_description("A friendly chat room").is_ok());
    assert!(validate_room_description("这是一个友好的聊天室").is_ok());
  }

  #[test]
  fn test_validate_room_description_too_long() {
    let long_desc = "a".repeat(501);
    assert!(validate_room_description(&long_desc).is_err());

    let max_desc = "a".repeat(500);
    assert!(validate_room_description(&max_desc).is_ok());
  }

  // ==========================================================================
  // Room Password Validation Tests
  // ==========================================================================

  #[test]
  fn test_validate_room_password_valid() {
    assert!(validate_room_password("").is_ok()); // Empty = no password
    assert!(validate_room_password("password123").is_ok());
    assert!(validate_room_password("复杂密码!@#").is_ok());
    let max_pass = "a".repeat(64);
    assert!(validate_room_password(&max_pass).is_ok());
  }

  #[test]
  fn test_validate_room_password_too_long() {
    let long_pass = "a".repeat(65);
    assert!(validate_room_password(&long_pass).is_err());
  }

  // ==========================================================================
  // Announcement Validation Tests
  // ==========================================================================

  #[test]
  fn test_validate_announcement_valid() {
    assert!(validate_announcement("Welcome to the room!").is_ok());
    assert!(validate_announcement("欢迎来到聊天室！").is_ok());
    assert!(validate_announcement("Important: Please read the rules.").is_ok());
  }

  #[test]
  fn test_validate_announcement_empty() {
    assert!(validate_announcement("").is_err());
    assert!(validate_announcement("   ").is_err());
  }

  #[test]
  fn test_validate_announcement_too_long() {
    let long_announce = "a".repeat(501);
    assert!(validate_announcement(&long_announce).is_err());

    let max_announce = "a".repeat(500);
    assert!(validate_announcement(&max_announce).is_ok());
  }

  // ==========================================================================
  // Danmaku Validation Tests
  // ==========================================================================

  #[test]
  fn test_validate_danmaku_valid() {
    assert!(validate_danmaku("Hello!").is_ok());
    assert!(validate_danmaku("弹幕内容").is_ok());
    assert!(validate_danmaku("233333").is_ok());
  }

  #[test]
  fn test_validate_danmaku_empty() {
    assert!(validate_danmaku("").is_err());
    assert!(validate_danmaku("   ").is_err());
  }

  #[test]
  fn test_validate_danmaku_too_long() {
    let long_danmaku = "a".repeat(101);
    assert!(validate_danmaku(&long_danmaku).is_err());

    let max_danmaku = "a".repeat(100);
    assert!(validate_danmaku(&max_danmaku).is_ok());

    // Chinese characters
    let long_chinese = "弹".repeat(101);
    assert!(validate_danmaku(&long_chinese).is_err());
  }

  // ==========================================================================
  // Message Validation Tests
  // ==========================================================================

  #[test]
  fn test_validate_message_valid() {
    assert!(validate_message("Hello, world!").is_ok());
    assert!(validate_message("你好，世界！").is_ok());
    assert!(validate_message("a").is_ok());
    assert!(validate_message(&"a".repeat(10000)).is_ok());
  }

  #[test]
  fn test_validate_message_empty() {
    assert!(validate_message("").is_err());
    assert!(validate_message("   ").is_err());
    assert!(validate_message("\n\t").is_err());
  }

  #[test]
  fn test_validate_message_too_long() {
    let long_message = "a".repeat(10001);
    assert!(validate_message(&long_message).is_err());

    let max_message = "a".repeat(10000);
    assert!(validate_message(&max_message).is_ok());
  }

  // ==========================================================================
  // User ID Validation Tests
  // ==========================================================================

  #[test]
  fn test_validate_user_id_valid() {
    assert!(validate_user_id("550e8400-e29b-41d4-a716-446655440000").is_ok());
    assert!(validate_user_id("00000000-0000-0000-0000-000000000000").is_ok());
  }

  #[test]
  fn test_validate_user_id_invalid() {
    assert!(validate_user_id("").is_err());
    assert!(validate_user_id("not-a-uuid").is_err());
    assert!(validate_user_id("550e8400-e29b-41d4-a716").is_err());
    assert!(validate_user_id("550e8400-e29b-41d4-a716-446655440000-extra").is_err());
  }

  // ==========================================================================
  // Room ID Validation Tests
  // ==========================================================================

  #[test]
  fn test_validate_room_id_valid() {
    assert!(validate_room_id("550e8400-e29b-41d4-a716-446655440000").is_ok());
    assert!(validate_room_id("12345678-1234-1234-1234-123456789012").is_ok());
  }

  #[test]
  fn test_validate_room_id_invalid() {
    assert!(validate_room_id("").is_err());
    assert!(validate_room_id("not-a-uuid").is_err());
    assert!(validate_room_id("550e8400-e29b-41d4-a716").is_err());
  }

  // ==========================================================================
  // Error Code Roundtrip Tests
  // ==========================================================================

  #[test]
  fn test_error_code_bitcode_roundtrip() {
    let codes = [
      SIG001, SIG101, CHT001, CHT101, CHT103, AV401, ROM104, ROM108, THR103, THR104, FIL101,
      AUTH501, PST301, SYS301,
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
    let response = ErrorResponse::new(ROM104, "User already in room", "trace123")
      .with_detail("user_id", "abc123");

    let json = serde_json::to_string(&response).unwrap();
    let decoded: ErrorResponse = serde_json::from_str(&json).unwrap();

    assert_eq!(response.code, decoded.code);
    assert_eq!(response.message, decoded.message);
    assert_eq!(response.i18n_key, decoded.i18n_key);
    assert_eq!(response.details, decoded.details);
    assert_eq!(response.trace_id, decoded.trace_id);
  }

  // ==========================================================================
  // Validation Error Tests
  // ==========================================================================

  #[test]
  fn test_validation_error_display() {
    let err = validate_username("ab").unwrap_err();
    let display = format!("{err}");
    assert!(display.contains("username"));
  }

  #[test]
  fn test_validation_error_fields() {
    let err = validate_message("").unwrap_err();
    assert_eq!(err.field, "message");
    assert_eq!(err.code, CHT105);
  }
}
