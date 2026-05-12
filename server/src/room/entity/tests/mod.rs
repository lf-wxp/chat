//! Unit tests for Room entity.
//!
//! Tests cover room creation, password management, member management,
//! ban/unban, mute/unmute, role management, ownership transfer,
//! announcement, nickname, and utility methods.

mod announcement;
mod ban;
mod creation;
mod member;
mod mute;
mod nickname;
mod ownership;
mod password;
mod role;
mod utility;

use super::*;

/// Create a test room with default settings.
pub(crate) fn create_test_room() -> Room {
  let room_id = RoomId::new();
  let owner_id = UserId::new();
  Room::new(room_id, "Test Room".to_string(), RoomType::Chat, owner_id)
}
