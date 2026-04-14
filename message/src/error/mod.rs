//! Error code system for WebRTC Chat Application.
//!
//! This module provides a unified error code system with i18n key mapping
//! and input validation utilities.

pub mod codes;
pub mod validation;

use bitcode::{Decode, Encode};
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

// Re-export key types and constants from submodules
pub use codes::*;
pub use validation::{
  ValidationError, ValidationResult, max_lengths, validate_announcement, validate_danmaku,
  validate_message, validate_nickname, validate_room_description, validate_room_id,
  validate_room_name, validate_room_password, validate_user_id, validate_username,
};

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
  /// Sequence number within module-category (supports multi-digit codes like ROM1201)
  pub sequence: u16,
}

impl ErrorCode {
  /// Create a new error code.
  #[must_use]
  pub const fn new(module: ErrorModule, category: ErrorCategory, sequence: u16) -> Self {
    Self {
      module,
      category,
      sequence,
    }
  }

  /// Convert to string format (e.g., "SIG001", "SIG101", "ROM1201").
  ///
  /// The numeric part is computed as `category * 100 + sequence`, then
  /// zero-padded to at least 3 digits.
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
    let numeric = self.category as u16 * 100 + self.sequence;
    format!("{module_str}{numeric:03}")
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
    serialize_with = "serde_helpers::serialize_timestamp_nanos",
    deserialize_with = "serde_helpers::deserialize_timestamp_nanos"
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

// ============================================================================
// Serde Helpers
// ============================================================================

/// Serde helper functions for custom serialization.
#[allow(clippy::trivially_copy_pass_by_ref)]
mod serde_helpers {
  use chrono::{DateTime, TimeZone, Utc};
  use serde::{Deserialize, Deserializer, Serializer};

  /// Serialize timestamp nanos to ISO 8601 string.
  pub(super) fn serialize_timestamp_nanos<S>(nanos: &i64, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let dt = Utc.timestamp_nanos(*nanos);
    serializer.serialize_str(&dt.to_rfc3339())
  }

  /// Deserialize ISO 8601 string to timestamp nanos.
  pub(super) fn deserialize_timestamp_nanos<'de, D>(deserializer: D) -> Result<i64, D::Error>
  where
    D: Deserializer<'de>,
  {
    let s = String::deserialize(deserializer)?;
    let dt = DateTime::parse_from_rfc3339(&s).map_err(serde::de::Error::custom)?;
    Ok(dt.timestamp_nanos_opt().unwrap_or(0))
  }
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
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests;
