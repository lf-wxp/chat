//! Room state unit tests.

mod creation;
mod edge_cases;
mod moderation;
mod role_management;
mod room_lifecycle;
mod validation;

use message::signaling::CreateRoom;
use message::types::{RoomType, UserId};

/// Helper: create a basic `CreateRoom` request.
pub(super) fn create_room_request() -> CreateRoom {
  CreateRoom {
    name: "test-room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  }
}

/// Helper: create a `CreateRoom` request with a password.
pub(super) fn create_room_request_with_password(password: &str) -> CreateRoom {
  CreateRoom {
    name: "test-room".to_string(),
    room_type: RoomType::Chat,
    password: Some(password.to_string()),
    max_participants: 8,
  }
}

/// Helper: create a `CreateRoom` request with custom name and room type.
pub(super) fn create_room_request_with_params(name: &str, room_type: RoomType) -> CreateRoom {
  CreateRoom {
    name: name.to_string(),
    room_type,
    password: None,
    max_participants: 8,
  }
}

/// Helper: create a deterministic `UserId` for tests.
pub(super) fn test_user_id(n: u64) -> UserId {
  UserId::from(n)
}
