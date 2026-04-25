//! Room lifecycle tests: create, join, leave.

use message::signaling::{JoinRoom, LeaveRoom};
use message::types::RoomType;

use super::super::RoomState;
use super::*;

// ===========================================================================
// Create Room
// ===========================================================================

#[test]
fn test_create_room() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let request = create_room_request();

  let result = state.create_room(&request, owner_id.clone());
  assert!(result.is_ok());

  let (_room_id, room_info) = result.unwrap();
  assert_eq!(room_info.name, "test-room");
  assert_eq!(room_info.room_type, RoomType::Chat);
  assert_eq!(room_info.owner_id, owner_id);
  assert!(!room_info.is_password_protected());
  assert_eq!(state.room_count(), 1);
}

#[test]
fn test_create_room_with_password() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let request = create_room_request_with_password("secret");

  let result = state.create_room(&request, owner_id.clone());
  assert!(result.is_ok());

  let (_, room_info) = result.unwrap();
  assert!(room_info.is_password_protected());
}

#[test]
fn test_create_room_with_max_participants() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let mut request = create_room_request();
  request.max_participants = 4;

  let result = state.create_room(&request, owner_id.clone());
  assert!(result.is_ok());

  let (_, room_info) = result.unwrap();
  assert_eq!(room_info.max_members, 4);
}

#[test]
fn test_create_room_max_participants_clamped() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let mut request = create_room_request();
  // Value above the maximum (8) should be clamped
  request.max_participants = 20;

  let result = state.create_room(&request, owner_id.clone());
  assert!(result.is_ok());

  let (_, room_info) = result.unwrap();
  assert_eq!(room_info.max_members, 8);
}

#[test]
fn test_create_room_max_participants_minimum() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let mut request = create_room_request();
  // Value below the minimum (2) should be clamped
  request.max_participants = 1;

  let result = state.create_room(&request, owner_id.clone());
  assert!(result.is_ok());

  let (_, room_info) = result.unwrap();
  assert_eq!(room_info.max_members, 2);
}

#[test]
fn test_create_room_same_owner_same_type_fails() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let request = create_room_request();

  let result1 = state.create_room(&request, owner_id.clone());
  assert!(result1.is_ok());

  // Same owner, same room type should fail
  let result2 = state.create_room(&request, owner_id.clone());
  assert!(result2.is_err());
}

#[test]
fn test_create_room_same_owner_different_type_ok() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);

  let request_chat = CreateRoom {
    name: "chat-room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let request_theater = CreateRoom {
    name: "theater-room".to_string(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 8,
  };

  let result1 = state.create_room(&request_chat, owner_id.clone());
  assert!(result1.is_ok());

  // Same owner, different room type should succeed
  let result2 = state.create_room(&request_theater, owner_id.clone());
  assert!(result2.is_ok());
}

#[test]
fn test_create_room_password_too_short() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let request = CreateRoom {
    name: "test-room".to_string(),
    room_type: RoomType::Chat,
    password: Some("ab".to_string()), // Too short (< 4 chars)
    max_participants: 8,
  };

  let result = state.create_room(&request, owner_id);
  assert!(result.is_err());
}

#[test]
fn test_create_room_empty_name_fails() {
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

// ===========================================================================
// Join Room
// ===========================================================================

#[test]
fn test_join_room() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let member_id = test_user_id(2);
  let request = create_room_request();

  let (room_id, _) = state.create_room(&request, owner_id).unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  let result = state.join_room(&join_request, member_id.clone(), "member".to_string());
  assert!(result.is_ok());

  let (room_info, members) = result.unwrap();
  assert_eq!(room_info.member_count, 2);
  assert_eq!(members.len(), 2);
}

#[test]
fn test_join_room_already_in_room() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let request = create_room_request();

  let (room_id, _) = state.create_room(&request, owner_id.clone()).unwrap();

  // Owner tries to join again (already in room)
  let join_request = JoinRoom {
    room_id,
    password: None,
  };
  let result = state.join_room(&join_request, owner_id, "owner".to_string());
  assert!(result.is_err());
}

#[test]
fn test_join_room_not_found() {
  let state = RoomState::new();
  let user_id = test_user_id(1);
  let join_request = JoinRoom {
    room_id: message::types::RoomId::new(),
    password: None,
  };

  let result = state.join_room(&join_request, user_id, "user".to_string());
  assert!(result.is_err());
}

#[test]
fn test_join_room_with_password() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let member_id = test_user_id(2);
  let request = create_room_request_with_password("secret");

  let (room_id, _) = state.create_room(&request, owner_id).unwrap();

  // Join without password should fail
  let join_no_pw = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  let result = state.join_room(&join_no_pw, member_id.clone(), "member".to_string());
  assert!(result.is_err());

  // Join with correct password should succeed
  let join_with_pw = JoinRoom {
    room_id: room_id.clone(),
    password: Some("secret".to_string()),
  };
  let result = state.join_room(&join_with_pw, member_id, "member".to_string());
  assert!(result.is_ok());
}

#[test]
fn test_join_room_wrong_password() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let member_id = test_user_id(2);
  let request = create_room_request_with_password("secret");

  let (room_id, _) = state.create_room(&request, owner_id).unwrap();

  let join_request = JoinRoom {
    room_id,
    password: Some("wrong".to_string()),
  };
  let result = state.join_room(&join_request, member_id, "member".to_string());
  assert!(result.is_err());
}

#[test]
fn test_join_room_full() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let mut request = create_room_request();
  request.max_participants = 2;

  let (room_id, _) = state.create_room(&request, owner_id).unwrap();

  // Fill the room
  let member_id = test_user_id(2);
  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  let result = state.join_room(&join_request, member_id, "member".to_string());
  assert!(result.is_ok());

  // Third member should fail (room full)
  let extra_id = test_user_id(3);
  let join_request2 = JoinRoom {
    room_id,
    password: None,
  };
  let result2 = state.join_room(&join_request2, extra_id, "extra".to_string());
  assert!(result2.is_err());
}

// ===========================================================================
// Leave Room
// ===========================================================================

#[test]
fn test_leave_room() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let member_id = test_user_id(2);
  let request = create_room_request();

  let (room_id, _) = state.create_room(&request, owner_id).unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "member".to_string())
    .unwrap();

  // Member leaves
  let leave_request = LeaveRoom {
    room_id: room_id.clone(),
  };
  let result = state.leave_room(&leave_request, &member_id);
  assert!(result.is_ok());

  let leave_result = result.unwrap();
  assert!(!leave_result.room_destroyed);
  assert!(leave_result.room_info.is_some());
  assert_eq!(leave_result.members.len(), 1);
}

#[test]
fn test_leave_room_not_in_room() {
  let state = RoomState::new();
  let user_id = test_user_id(1);

  let leave_request = LeaveRoom {
    room_id: message::types::RoomId::new(),
  };
  let result = state.leave_room(&leave_request, &user_id);
  assert!(result.is_err());
}

#[test]
fn test_leave_room_owner_transfers_ownership() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let member_id = test_user_id(2);
  let request = create_room_request();

  let (room_id, _) = state.create_room(&request, owner_id.clone()).unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "member".to_string())
    .unwrap();

  // Owner leaves, ownership should transfer to member
  let leave_request = LeaveRoom {
    room_id: room_id.clone(),
  };
  let result = state.leave_room(&leave_request, &owner_id);
  assert!(result.is_ok());

  let leave_result = result.unwrap();
  assert!(leave_result.ownership_transfer.is_some());
  assert_eq!(leave_result.ownership_transfer.unwrap(), member_id);

  // Verify new owner
  let room = state.get_room(&room_id).unwrap();
  let new_owner = room.get_member(&member_id).unwrap();
  assert_eq!(new_owner.role, message::types::RoomRole::Owner);
}

#[test]
fn test_leave_room_destroys_when_empty() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let request = create_room_request();

  let (room_id, _) = state.create_room(&request, owner_id.clone()).unwrap();

  // Owner leaves - room should be destroyed (only member)
  let leave_request = LeaveRoom {
    room_id: room_id.clone(),
  };
  let result = state.leave_room(&leave_request, &owner_id);
  assert!(result.is_ok());

  let leave_result = result.unwrap();
  assert!(leave_result.room_destroyed);
  assert!(leave_result.room_info.is_none());
  assert!(leave_result.members.is_empty());
  assert_eq!(state.room_count(), 0);
}

#[test]
fn test_leave_room_wrong_room_id() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let request = create_room_request();

  let _ = state.create_room(&request, owner_id.clone()).unwrap();

  // Try to leave a different room
  let leave_request = LeaveRoom {
    room_id: message::types::RoomId::new(),
  };
  let result = state.leave_room(&leave_request, &owner_id);
  assert!(result.is_err());
}

// Duplicate tests that used incorrect API have been removed.
// The tests above already cover: create room (with/without password, max participants),
// join room (success, not found, already in room, with/wrong password, full),
// and leave room (success, owner transfers ownership, destroys when empty, wrong room id, not in room).
