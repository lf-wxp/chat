//! Room lifecycle tests (create, join, leave, destroy).

use super::*;

#[test]
fn test_create_room() {
  let state = create_test_room_state();
  let owner_id = message::types::UserId::new();

  let request = CreateRoom {
    name: "Test Room".to_string(),
    description: String::new(),
    room_type: message::types::RoomType::Chat,
    password: None,
    max_participants: 8,
  };

  let result = state.create_room(&request, owner_id.clone());
  assert!(result.is_ok());

  let (_room_id, room_info) = result.unwrap();
  assert_eq!(room_info.name, "Test Room");
  assert_eq!(room_info.room_type, message::types::RoomType::Chat);
  assert_eq!(room_info.owner_id, owner_id);
  assert!(!room_info.is_password_protected());
}

#[test]
fn test_create_room_with_password() {
  let state = create_test_room_state();
  let owner_id = message::types::UserId::new();

  let request = CreateRoom {
    name: "Private Room".to_string(),
    description: String::new(),
    room_type: message::types::RoomType::Chat,
    password: Some("secret123".to_string()),
    max_participants: 8,
  };

  let result = state.create_room(&request, owner_id);
  assert!(result.is_ok());

  let (_, room_info) = result.unwrap();
  assert!(room_info.is_password_protected());
}

#[test]
fn test_join_room() {
  let state = create_test_room_state();
  let owner_id = message::types::UserId::new();
  let member_id = message::types::UserId::new();

  // Create room
  let create_request = CreateRoom {
    name: "Test Room".to_string(),
    description: String::new(),
    room_type: message::types::RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = state.create_room(&create_request, owner_id).unwrap();

  // Join room
  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  let result = state.join_room(&join_request, member_id.clone(), "Member".to_string());
  assert!(result.is_ok());

  let (_, members) = result.unwrap();
  assert_eq!(members.len(), 2);
}

#[test]
fn test_join_password_protected_room() {
  let state = create_test_room_state();
  let owner_id = message::types::UserId::new();
  let member_id = message::types::UserId::new();

  // Create room with password
  let create_request = CreateRoom {
    name: "Private Room".to_string(),
    description: String::new(),
    room_type: message::types::RoomType::Chat,
    password: Some("secret123".to_string()),
    max_participants: 8,
  };
  let (room_id, _) = state.create_room(&create_request, owner_id).unwrap();

  // Try to join without password
  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  let result = state.join_room(&join_request, member_id.clone(), "Member".to_string());
  assert!(result.is_err());

  // Join with correct password
  let join_request = JoinRoom {
    room_id,
    password: Some("secret123".to_string()),
  };
  let result = state.join_room(&join_request, member_id, "Member".to_string());
  assert!(result.is_ok());
}

#[test]
fn test_leave_room() {
  let state = create_test_room_state();
  let owner_id = message::types::UserId::new();
  let member_id = message::types::UserId::new();

  // Create and join room
  let create_request = CreateRoom {
    name: "Test Room".to_string(),
    description: String::new(),
    room_type: message::types::RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = state
    .create_room(&create_request, owner_id.clone())
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Member".to_string())
    .unwrap();

  // Leave room
  let leave_request = LeaveRoom {
    room_id: room_id.clone(),
  };
  let result = state.leave_room(&leave_request, &member_id);
  assert!(result.is_ok());

  let leave_result = result.unwrap();
  assert!(!leave_result.room_destroyed);
  assert_eq!(leave_result.members.len(), 1);
}

#[test]
fn test_ownership_transfer_on_owner_leave() {
  let state = create_test_room_state();
  let owner_id = message::types::UserId::new();
  let member_id = message::types::UserId::new();

  // Create room
  let create_request = CreateRoom {
    name: "Test Room".to_string(),
    description: String::new(),
    room_type: message::types::RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = state
    .create_room(&create_request, owner_id.clone())
    .unwrap();

  // Member joins
  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Member".to_string())
    .unwrap();

  // Owner leaves
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
  assert_eq!(room.info.owner_id, member_id);
}

#[test]
fn test_room_destruction_when_empty() {
  let state = create_test_room_state();
  let owner_id = message::types::UserId::new();

  // Create room
  let create_request = CreateRoom {
    name: "Test Room".to_string(),
    description: String::new(),
    room_type: message::types::RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = state
    .create_room(&create_request, owner_id.clone())
    .unwrap();

  // Owner leaves (only member)
  let leave_request = LeaveRoom {
    room_id: room_id.clone(),
  };
  let result = state.leave_room(&leave_request, &owner_id);
  assert!(result.is_ok());

  let leave_result = result.unwrap();
  assert!(leave_result.room_destroyed);

  // Room should no longer exist
  assert!(state.get_room(&room_id).is_none());
}

#[test]
fn test_room_full() {
  let state = create_test_room_state();
  let owner_id = message::types::UserId::new();

  // Create room with max 2 members
  let create_request = CreateRoom {
    name: "Small Room".to_string(),
    description: String::new(),
    room_type: message::types::RoomType::Chat,
    password: None,
    max_participants: 2,
  };
  let (room_id, _) = state
    .create_room(&create_request, owner_id.clone())
    .unwrap();

  // Join with second member
  let member1 = message::types::UserId::new();
  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member1, "M1".to_string())
    .unwrap();

  // Try to join with third member (should fail)
  let member2 = message::types::UserId::new();
  let join_request = JoinRoom {
    room_id,
    password: None,
  };
  let result = state.join_room(&join_request, member2, "M2".to_string());
  assert_eq!(result.unwrap_err(), RoomError::RoomFull);
}

#[test]
fn test_transfer_ownership() {
  let state = create_test_room_state();
  let owner_id = message::types::UserId::new();
  let member_id = message::types::UserId::new();

  // Create and join room
  let create_request = CreateRoom {
    name: "Test Room".to_string(),
    description: String::new(),
    room_type: message::types::RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = state
    .create_room(&create_request, owner_id.clone())
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Member".to_string())
    .unwrap();

  // Transfer ownership
  let transfer_request = TransferOwnership {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = state.transfer_ownership(&transfer_request, &owner_id);
  assert!(result.is_ok());

  // Verify new owner
  let room = state.get_room(&room_id).unwrap();
  assert_eq!(room.info.owner_id, member_id);

  // Old owner should now be admin
  let old_owner = room.get_member(&owner_id).unwrap();
  assert_eq!(old_owner.role, message::types::RoomRole::Admin);

  // New owner should have owner role
  let new_owner = room.get_member(&member_id).unwrap();
  assert_eq!(new_owner.role, message::types::RoomRole::Owner);
}
