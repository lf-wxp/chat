//! Room name validation tests.

use message::signaling::CreateRoom;
use message::types::RoomType;

use super::super::RoomState;
use super::*;

#[test]
fn test_room_name_too_short() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  // Single character name - but validation checks empty, not length minimum
  let request = CreateRoom {
    name: "a".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };

  // Single char name is valid per current validation rules
  let result = state.create_room(&request, owner_id);
  assert!(result.is_ok());
}

#[test]
fn test_room_name_too_long() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let long_name = "a".repeat(101); // Exceeds MAX_ROOM_NAME_LENGTH (100)
  let request = CreateRoom {
    name: long_name,
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };

  let result = state.create_room(&request, owner_id);
  assert!(result.is_err());
}

#[test]
fn test_room_name_empty() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let request = CreateRoom {
    name: "".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };

  let result = state.create_room(&request, owner_id);
  assert!(result.is_err());
}

#[test]
fn test_room_name_maximum_length() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let max_name = "a".repeat(100); // Exactly MAX_ROOM_NAME_LENGTH
  let request = CreateRoom {
    name: max_name,
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };

  let result = state.create_room(&request, owner_id);
  assert!(result.is_ok());
}

#[test]
fn test_room_name_with_leading_whitespace() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let request = CreateRoom {
    name: " room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };

  let result = state.create_room(&request, owner_id);
  assert!(result.is_err());
}

#[test]
fn test_room_name_with_trailing_whitespace() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let request = CreateRoom {
    name: "room ".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };

  let result = state.create_room(&request, owner_id);
  assert!(result.is_err());
}

#[test]
fn test_room_name_consecutive_spaces() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let request = CreateRoom {
    name: "my  room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };

  let result = state.create_room(&request, owner_id);
  assert!(result.is_err());
}

#[test]
fn test_room_name_invalid_characters() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let request = CreateRoom {
    name: "room@#".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };

  let result = state.create_room(&request, owner_id);
  assert!(result.is_err());
}

#[test]
fn test_room_name_valid_with_hyphen() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let request = CreateRoom {
    name: "my-room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };

  let result = state.create_room(&request, owner_id);
  assert!(result.is_ok());
}

#[test]
fn test_room_name_valid_with_underscore() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let request = CreateRoom {
    name: "my_room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };

  let result = state.create_room(&request, owner_id);
  assert!(result.is_ok());
}

#[test]
fn test_room_password_minimum_length() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let request = CreateRoom {
    name: "test-room".to_string(),
    room_type: RoomType::Chat,
    password: Some("1234".to_string()),
    max_participants: 8,
  };

  let result = state.create_room(&request, owner_id);
  assert!(result.is_ok());
}

#[test]
fn test_room_password_too_short() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let request = CreateRoom {
    name: "test-room".to_string(),
    room_type: RoomType::Chat,
    password: Some("ab".to_string()),
    max_participants: 8,
  };

  let result = state.create_room(&request, owner_id);
  assert!(result.is_err());
}
