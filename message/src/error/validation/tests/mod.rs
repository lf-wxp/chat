//! Input validation tests module.
//!
//! Tests are organized by functionality:
//! - `field_validation`: Basic field validation tests (username, nickname, room, etc.)
//! - `unicode_and_xss`: Unicode character, XSS filter, and script injection tests
//! - `boundary_and_security`: Boundary length, `DoS`, and security edge case tests

mod boundary_and_security;
mod field_validation;
mod unicode_and_xss;

// Re-export all necessary types for test submodules
pub(super) use super::{
  CHT105, validate_announcement, validate_danmaku, validate_message, validate_nickname,
  validate_room_description, validate_room_id, validate_room_name, validate_room_password,
  validate_user_id, validate_username,
};
