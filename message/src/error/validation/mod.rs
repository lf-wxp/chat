//! Input validation utilities.
//!
//! Provides validation functions for usernames, nicknames, room names,
//! messages, and other user inputs with consistent error reporting.

use super::ErrorCode;
use super::codes::{AUTH101, CHT101, CHT105, ROM101, ROM105, THR103};

// ============================================================================
// Maximum Length Constants
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

// ============================================================================
// Validation Error Type
// ============================================================================

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

// ============================================================================
// Validation Functions
// ============================================================================

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

  let len = password.chars().count();

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
mod tests;
