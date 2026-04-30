//! Edge cases and error handling tests.

use super::*;

#[test]
fn test_join_nonexistent_room() {
  let state = create_test_room_state();
  let user_id = UserId::new();

  let join_request = JoinRoom {
    room_id: RoomId::new(),
    password: None,
  };
  let result = state.join_room(&join_request, user_id, "User".to_string());
  assert_eq!(result.unwrap_err(), RoomError::RoomNotFound);
}

#[test]
fn test_leave_nonexistent_room() {
  let state = create_test_room_state();
  let user_id = UserId::new();

  let leave_request = LeaveRoom {
    room_id: RoomId::new(),
  };
  let result = state.leave_room(&leave_request, &user_id);
  assert_eq!(result.unwrap_err(), RoomError::UserNotInRoom);
}

#[test]
fn test_kick_nonexistent_member() {
  let state = create_test_room_state();
  let owner_id = UserId::new();
  let nonexistent_member = UserId::new();

  // Create room
  let create_request = CreateRoom {
    name: "Test Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = state
    .create_room(&create_request, owner_id.clone())
    .unwrap();

  // Try to kick nonexistent member
  let kick_request = KickMember {
    room_id: room_id.clone(),
    target: nonexistent_member.clone(),
  };
  let result = state.kick_member(&kick_request, &owner_id);
  assert_eq!(result.unwrap_err(), RoomError::RoomNotFound);
}

#[test]
fn test_promote_nonexistent_member() {
  let state = create_test_room_state();
  let owner_id = UserId::new();
  let nonexistent_member = UserId::new();

  // Create room
  let create_request = CreateRoom {
    name: "Test Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = state
    .create_room(&create_request, owner_id.clone())
    .unwrap();

  // Try to promote nonexistent member
  let promote_request = PromoteAdmin {
    room_id: room_id.clone(),
    target: nonexistent_member.clone(),
  };
  let result = state.promote_admin(&promote_request, &owner_id);
  assert_eq!(result.unwrap_err(), RoomError::RoomNotFound);
}

#[test]
fn test_already_member() {
  let state = create_test_room_state();
  let owner_id = UserId::new();

  // Create room
  let create_request = CreateRoom {
    name: "Test Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = state
    .create_room(&create_request, owner_id.clone())
    .unwrap();

  // Try to join again (owner is already a member)
  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  let result = state.join_room(&join_request, owner_id.clone(), "Owner".to_string());
  assert_eq!(result.unwrap_err(), RoomError::UserAlreadyInRoom);
}

#[test]
fn test_already_admin() {
  let state = create_test_room_state();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  // Create and join room
  let create_request = CreateRoom {
    name: "Test Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
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

  // Promote to admin
  let promote_request = PromoteAdmin {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  state.promote_admin(&promote_request, &owner_id).unwrap();

  // Try to promote again
  let result = state.promote_admin(&promote_request, &owner_id);
  assert_eq!(result.unwrap_err(), RoomError::InsufficientPermission);
}

#[test]
fn test_demote_regular_member() {
  let state = create_test_room_state();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  // Create and join room
  let create_request = CreateRoom {
    name: "Test Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
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

  // Try to demote regular member (not an admin)
  let demote_request = DemoteAdmin {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = state.demote_admin(&demote_request, &owner_id);
  assert_eq!(result.unwrap_err(), RoomError::InsufficientPermission);
}

#[test]
fn test_cannot_demote_owner() {
  let state = create_test_room_state();
  let owner_id = UserId::new();

  // Create room
  let create_request = CreateRoom {
    name: "Test Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = state
    .create_room(&create_request, owner_id.clone())
    .unwrap();

  // Try to demote owner
  let demote_request = DemoteAdmin {
    room_id: room_id.clone(),
    target: owner_id.clone(),
  };
  let result = state.demote_admin(&demote_request, &owner_id);
  assert_eq!(result.unwrap_err(), RoomError::InsufficientPermission);
}

#[test]
fn test_create_duplicate_room_name() {
  let state = create_test_room_state();
  let owner1_id = UserId::new();
  let owner2_id = UserId::new();

  // Create first room
  let create_request = CreateRoom {
    name: "Duplicate Name".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let result = state.create_room(&create_request, owner1_id);
  assert!(result.is_ok());

  // Create second room with same name (should succeed - names don't need to be unique)
  let create_request = CreateRoom {
    name: "Duplicate Name".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let result = state.create_room(&create_request, owner2_id);
  assert!(result.is_ok());

  // Verify two different rooms exist
  let _room1 = result.unwrap();
  let room2 = state.create_room(&create_request, UserId::new());
  assert!(room2.is_ok());
}

#[test]
fn test_password_validation_wrong_password() {
  let state = create_test_room_state();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  // Create room with password
  let create_request = CreateRoom {
    name: "Protected Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: Some("correct-password".to_string()),
    max_participants: 8,
  };
  let (room_id, _) = state.create_room(&create_request, owner_id).unwrap();

  // Try to join with wrong password
  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: Some("wrong-password".to_string()),
  };
  let result = state.join_room(&join_request, member_id, "Member".to_string());
  assert_eq!(
    result.unwrap_err(),
    RoomError::InvalidPassword("Incorrect password".to_string())
  );
}

#[test]
fn test_operations_on_nonexistent_room() {
  let state = create_test_room_state();
  let user_id = UserId::new();
  let target_id = UserId::new();

  // All operations on nonexistent room should fail
  let room_id = RoomId::new();

  // Kick
  let kick_request = KickMember {
    room_id: room_id.clone(),
    target: target_id.clone(),
  };
  assert_eq!(
    state.kick_member(&kick_request, &user_id).unwrap_err(),
    RoomError::RoomNotFound
  );

  // Mute
  let mute_request = MuteMember {
    room_id: room_id.clone(),
    target: target_id.clone(),
    duration_secs: Some(60),
  };
  assert_eq!(
    state.mute_member(&mute_request, &user_id).unwrap_err(),
    RoomError::RoomNotFound
  );

  // Ban
  let ban_request = BanMember {
    room_id: room_id.clone(),
    target: target_id.clone(),
  };
  assert_eq!(
    state.ban_member(&ban_request, &user_id).unwrap_err(),
    RoomError::RoomNotFound
  );

  // Promote
  let promote_request = PromoteAdmin {
    room_id: room_id.clone(),
    target: target_id.clone(),
  };
  assert_eq!(
    state.promote_admin(&promote_request, &user_id).unwrap_err(),
    RoomError::RoomNotFound
  );

  // Transfer ownership
  let transfer_request = TransferOwnership {
    room_id: room_id.clone(),
    target: target_id.clone(),
  };
  assert_eq!(
    state
      .transfer_ownership(&transfer_request, &user_id)
      .unwrap_err(),
    RoomError::RoomNotFound
  );
}

#[test]
fn test_get_room_info() {
  let state = create_test_room_state();
  let owner_id = UserId::new();

  // Create room
  let create_request = CreateRoom {
    name: "Test Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, room_info) = state
    .create_room(&create_request, owner_id.clone())
    .unwrap();

  // Verify room info
  assert_eq!(room_info.name, "Test Room");
  assert_eq!(room_info.room_type, RoomType::Chat);
  assert_eq!(room_info.owner_id, owner_id);
  assert!(!room_info.is_password_protected());

  // Get room and verify
  let room = state.get_room(&room_id).unwrap();
  assert_eq!(room.info.name, "Test Room");
}

#[test]
fn test_get_nonexistent_room() {
  let state = create_test_room_state();
  let result = state.get_room(&RoomId::new());
  assert!(result.is_none());
}
