use super::*;
use crate::ws::tests::create_test_ws_state;
use message::RoomId;
use message::signaling::*;
use message::types::RoomType;

// ===== Create Room Tests =====

#[test]
fn test_create_room_success() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();

  let create_room = CreateRoom {
    name: "Test Room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };

  let result = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone());
  assert!(result.is_ok());

  let (room_id, room_info) = result.unwrap();
  assert!(ws_state.room_state.get_room(&room_id).is_some());
  assert_eq!(room_info.owner_id, owner_id);
}

#[test]
fn test_create_room_with_password() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();

  let create_room = CreateRoom {
    name: "Private Room".to_string(),
    room_type: RoomType::Chat,
    password: Some("secret123".to_string()),
    max_participants: 8,
  };

  let result = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone());
  assert!(result.is_ok());

  let (_, room_info) = result.unwrap();
  assert!(room_info.password_hash.is_some());
}

#[test]
fn test_create_room_user_already_owner_of_same_type() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();

  let create_room = CreateRoom {
    name: "Room 1".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };

  // Create first room
  ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  // Try to create second room of same type
  let create_room2 = CreateRoom {
    name: "Room 2".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };

  let result = ws_state
    .room_state
    .create_room(&create_room2, owner_id.clone());
  assert!(result.is_err());
  assert!(matches!(
    result.unwrap_err(),
    crate::room::RoomError::AlreadyOwnerOfSameType
  ));
}

#[test]
fn test_create_different_room_types() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();

  // Create Chat room
  let create_chat = CreateRoom {
    name: "Chat Room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  assert!(
    ws_state
      .room_state
      .create_room(&create_chat, owner_id.clone())
      .is_ok()
  );

  // Create Theater room (should succeed - different type)
  let create_theater = CreateRoom {
    name: "Theater Room".to_string(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 50,
  };
  assert!(
    ws_state
      .room_state
      .create_room(&create_theater, owner_id.clone())
      .is_ok()
  );
}

// ===== Join Room Tests =====

#[test]
fn test_join_room_success() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let joiner_id = UserId::new();

  // Register users first
  let _ = ws_state.user_store.register("owner", "password");
  let _ = ws_state.user_store.register("joiner", "password");

  // Create room
  let create_room = CreateRoom {
    name: "Test Room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  // Join room
  let join_room = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };

  let result = ws_state
    .room_state
    .join_room(&join_room, joiner_id.clone(), "joiner".to_string());
  assert!(result.is_ok());
}

#[test]
fn test_join_room_with_password() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let joiner_id = UserId::new();

  // Create room with password
  let create_room = CreateRoom {
    name: "Private Room".to_string(),
    room_type: RoomType::Chat,
    password: Some("secret123".to_string()),
    max_participants: 8,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  // Join with correct password
  let join_room = JoinRoom {
    room_id: room_id.clone(),
    password: Some("secret123".to_string()),
  };

  let result = ws_state
    .room_state
    .join_room(&join_room, joiner_id.clone(), "joiner".to_string());
  assert!(result.is_ok());
}

#[test]
fn test_join_room_wrong_password() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let joiner_id = UserId::new();

  // Create room with password
  let create_room = CreateRoom {
    name: "Private Room".to_string(),
    room_type: RoomType::Chat,
    password: Some("secret123".to_string()),
    max_participants: 8,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  // Join with wrong password
  let join_room = JoinRoom {
    room_id: room_id.clone(),
    password: Some("wrongpassword".to_string()),
  };

  let result = ws_state
    .room_state
    .join_room(&join_room, joiner_id.clone(), "joiner".to_string());
  assert!(result.is_err());
  assert!(matches!(
    result.unwrap_err(),
    crate::room::RoomError::InvalidPassword(_)
  ));
}

#[test]
fn test_join_room_not_found() {
  let ws_state = create_test_ws_state();
  let user_id = UserId::new();
  let room_id = RoomId::new();

  let join_room = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };

  let result = ws_state
    .room_state
    .join_room(&join_room, user_id.clone(), "user".to_string());
  assert!(result.is_err());
  assert!(matches!(
    result.unwrap_err(),
    crate::room::RoomError::RoomNotFound
  ));
}

// ===== Leave Room Tests =====

#[test]
fn test_leave_room_success() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  // Create room
  let create_room = CreateRoom {
    name: "Test Room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  // Member joins
  let join_room = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  ws_state
    .room_state
    .join_room(&join_room, member_id.clone(), "member".to_string())
    .unwrap();

  // Member leaves
  let leave_room = LeaveRoom {
    room_id: room_id.clone(),
  };
  let result = ws_state.room_state.leave_room(&leave_room, &member_id);
  assert!(result.is_ok());
}

#[test]
fn test_leave_room_owner_destroys_room() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();

  // Create room
  let create_room = CreateRoom {
    name: "Test Room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  // Owner leaves
  let leave_room = LeaveRoom {
    room_id: room_id.clone(),
  };
  let result = ws_state.room_state.leave_room(&leave_room, &owner_id);
  assert!(result.is_ok());

  let leave_result = result.unwrap();
  assert!(leave_result.room_destroyed);
  assert!(ws_state.room_state.get_room(&room_id).is_none());
}

// ===== Kick Member Tests =====

#[test]
fn test_kick_member_as_owner() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  // Create room and add member
  let create_room = CreateRoom {
    name: "Test Room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  let join_room = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  ws_state
    .room_state
    .join_room(&join_room, member_id.clone(), "member".to_string())
    .unwrap();

  // Owner kicks member
  let kick_member = KickMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = ws_state.room_state.kick_member(&kick_member, &owner_id);
  assert!(result.is_ok());
}

#[test]
fn test_kick_member_insufficient_permission() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let member1_id = UserId::new();
  let member2_id = UserId::new();

  // Create room and add members
  let create_room = CreateRoom {
    name: "Test Room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  let join_room = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  ws_state
    .room_state
    .join_room(&join_room, member1_id.clone(), "member1".to_string())
    .unwrap();
  ws_state
    .room_state
    .join_room(&join_room, member2_id.clone(), "member2".to_string())
    .unwrap();

  // Member1 tries to kick member2 (should fail)
  let kick_member = KickMember {
    room_id: room_id.clone(),
    target: member2_id.clone(),
  };
  let result = ws_state.room_state.kick_member(&kick_member, &member1_id);
  assert!(result.is_err());
  assert!(matches!(
    result.unwrap_err(),
    crate::room::RoomError::InsufficientPermission
  ));
}

// ===== Mute/Unmute Tests =====

#[test]
fn test_mute_member_success() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  // Create room and add member
  let create_room = CreateRoom {
    name: "Test Room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  let join_room = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  ws_state
    .room_state
    .join_room(&join_room, member_id.clone(), "member".to_string())
    .unwrap();

  // Mute member
  let mute_member = MuteMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
    duration_secs: Some(300),
  };
  let result = ws_state.room_state.mute_member(&mute_member, &owner_id);
  assert!(result.is_ok());
}

#[test]
fn test_unmute_member_success() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  // Create room, add member, and mute
  let create_room = CreateRoom {
    name: "Test Room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  let join_room = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  ws_state
    .room_state
    .join_room(&join_room, member_id.clone(), "member".to_string())
    .unwrap();

  let mute_member = MuteMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
    duration_secs: Some(300),
  };
  ws_state
    .room_state
    .mute_member(&mute_member, &owner_id)
    .unwrap();

  // Unmute member
  let unmute_member = UnmuteMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = ws_state.room_state.unmute_member(&unmute_member, &owner_id);
  assert!(result.is_ok());
}

// ===== Ban/Unban Tests =====

#[test]
fn test_ban_member_success() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  // Create room and add member
  let create_room = CreateRoom {
    name: "Test Room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  let join_room = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  ws_state
    .room_state
    .join_room(&join_room, member_id.clone(), "member".to_string())
    .unwrap();

  // Ban member
  let ban_member = BanMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = ws_state.room_state.ban_member(&ban_member, &owner_id);
  assert!(result.is_ok());
}

#[test]
fn test_banned_user_cannot_rejoin() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  // Create room
  let create_room = CreateRoom {
    name: "Test Room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  // First add member to room
  let join_room = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  ws_state
    .room_state
    .join_room(&join_room.clone(), member_id.clone(), "member".to_string())
    .unwrap();

  // Now ban the member
  let ban_member = BanMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  ws_state
    .room_state
    .ban_member(&ban_member, &owner_id)
    .unwrap();

  // Banned user tries to rejoin
  let result = ws_state
    .room_state
    .join_room(&join_room, member_id.clone(), "member".to_string());
  assert!(result.is_err());
  assert!(matches!(
    result.unwrap_err(),
    crate::room::RoomError::UserBanned
  ));
}

// ===== Promote/Demote Admin Tests =====

#[test]
fn test_promote_admin_success() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  // Create room and add member
  let create_room = CreateRoom {
    name: "Test Room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  let join_room = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  ws_state
    .room_state
    .join_room(&join_room, member_id.clone(), "member".to_string())
    .unwrap();

  // Promote to admin
  let promote_admin = PromoteAdmin {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = ws_state.room_state.promote_admin(&promote_admin, &owner_id);
  assert!(result.is_ok());
}

#[test]
fn test_demote_admin_success() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  // Create room, add member, promote to admin
  let create_room = CreateRoom {
    name: "Test Room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  let join_room = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  ws_state
    .room_state
    .join_room(&join_room, member_id.clone(), "member".to_string())
    .unwrap();

  let promote_admin = PromoteAdmin {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  ws_state
    .room_state
    .promote_admin(&promote_admin, &owner_id)
    .unwrap();

  // Demote back to member
  let demote_admin = DemoteAdmin {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = ws_state.room_state.demote_admin(&demote_admin, &owner_id);
  assert!(result.is_ok());
}

// ===== Transfer Ownership Tests =====

#[test]
fn test_transfer_ownership_success() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let new_owner_id = UserId::new();

  // Create room and add member
  let create_room = CreateRoom {
    name: "Test Room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  let join_room = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  ws_state
    .room_state
    .join_room(&join_room, new_owner_id.clone(), "newowner".to_string())
    .unwrap();

  // Transfer ownership
  let transfer_ownership = TransferOwnership {
    room_id: room_id.clone(),
    target: new_owner_id.clone(),
  };
  let result = ws_state
    .room_state
    .transfer_ownership(&transfer_ownership, &owner_id);
  assert!(result.is_ok());

  // Verify new owner
  let room = ws_state.room_state.get_room(&room_id).unwrap();
  assert_eq!(room.owner_id(), &new_owner_id);
}

#[test]
fn test_transfer_ownership_non_owner_fails() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let member_id = UserId::new();
  let target_id = UserId::new();

  // Create room and add members
  let create_room = CreateRoom {
    name: "Test Room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  let join_room = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  ws_state
    .room_state
    .join_room(&join_room, member_id.clone(), "member".to_string())
    .unwrap();
  ws_state
    .room_state
    .join_room(&join_room, target_id.clone(), "target".to_string())
    .unwrap();

  // Non-owner tries to transfer ownership
  let transfer_ownership = TransferOwnership {
    room_id: room_id.clone(),
    target: target_id.clone(),
  };
  let result = ws_state
    .room_state
    .transfer_ownership(&transfer_ownership, &member_id);
  assert!(result.is_err());
  assert!(matches!(
    result.unwrap_err(),
    crate::room::RoomError::InsufficientPermission
  ));
}
