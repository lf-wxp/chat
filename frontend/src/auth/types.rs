//! Auth type definitions.
//!
//! Contains request/response types and the AuthResult type used across
//! the auth module.

use serde::{Deserialize, Serialize};

/// Minimum password length for client-side validation.
///
/// Must stay in sync with the server-side policy in
/// `server/src/auth/validator.rs`. Changing this value requires updating
/// both sides and the i18n string `auth.password_too_short`.
pub(crate) const MIN_PASSWORD_LENGTH: usize = 8;

/// Registration request payload.
#[derive(Debug, Serialize)]
pub(crate) struct RegisterRequest {
  pub username: String,
  pub password: String,
}

/// Login request payload.
#[derive(Debug, Serialize)]
pub(crate) struct LoginRequest {
  pub username: String,
  pub password: String,
}

/// Auth API response.
#[derive(Debug, Deserialize)]
pub(crate) struct AuthResponse {
  pub user_id: String,
  pub token: String,
}

/// Auth API error response.
#[derive(Debug, Deserialize)]
pub(crate) struct AuthErrorResponse {
  pub error: String,
}

/// Result of an auth operation.
#[derive(Debug, Clone)]
pub struct AuthResult {
  /// Whether the operation succeeded.
  pub success: bool,
  /// Error message if the operation failed.
  pub error: Option<String>,
}

impl AuthResult {
  /// Create a successful result.
  pub fn ok() -> Self {
    Self {
      success: true,
      error: None,
    }
  }

  /// Create a failed result with an error message.
  pub fn err(msg: impl Into<String>) -> Self {
    Self {
      success: false,
      error: Some(msg.into()),
    }
  }
}
