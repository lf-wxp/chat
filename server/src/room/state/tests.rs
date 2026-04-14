use message::types::RoomType;

use super::*;

fn create_room_request(name: &str, room_type: RoomType) -> CreateRoom {
  CreateRoom {
    name: name.to_string(),
    room_type,
    max_participants: 8,
    password: None,
  }
}

fn create_room_request_with_password(name: &str, password: &str) -> CreateRoom {
  CreateRoom {
    name: name.to_string(),
    room_type: RoomType::Chat,
    max_participants: 8,
    password: Some(password.to_string()),
  }
}

// ===========================================================================
// RoomState creation
// ===========================================================================

#[test]
fn test_new_room_state_is_empty() {
  let state = RoomState::new();
  assert_eq!(state.room_count(), 0);
  assert_eq!(state.total_member_count(), 0);
  assert!(state.get_all_rooms().is_empty());
}

#[test]
fn test_default_room_state() {
  let state = RoomState::default();
  assert_eq!(state.room_count(), 0);
}

// ===========================================================================
// Room creation
// ===========================================================================

#[test]
fn test_create_room_success() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let request = create_room_request("Test Room", RoomType::Chat);

  let result = state.create_room(&request, owner_id.clone());
  assert!(result.is_ok());

  let (room_id, room_info) = result.unwrap();
  assert_eq!(room_info.name, "Test Room");
  assert_eq!(room_info.owner_id, owner_id);
  assert_eq!(state.room_count(), 1);
  // room_id is returned for caller use
  let _ = room_id;
}

#[test]
fn test_create_room_empty_name_rejected() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let request = CreateRoom {
    name: "".to_string(),
    room_type: RoomType::Chat,
    max_participants: 8,
    password: None,
  };

  let result = state.create_room(&request, owner_id);
  assert!(result.is_err());
  match result.unwrap_err() {
    RoomError::InvalidRoomName(msg) => assert!(msg.contains("empty")),
    _ => panic!("Expected InvalidRoomName error"),
  }
}

#[test]
fn test_create_room_name_too_long_rejected() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let request = CreateRoom {
    name: "a".repeat(MAX_ROOM_NAME_LENGTH + 1),
    room_type: RoomType::Chat,
    max_participants: 8,
    password: None,
  };

  let result = state.create_room(&request, owner_id);
  assert!(result.is_err());
}

#[test]
fn test_create_room_same_owner_same_type_rejected() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let request = create_room_request("Room 1", RoomType::Chat);

  state.create_room(&request, owner_id.clone()).unwrap();

  let request2 = create_room_request("Room 2", RoomType::Chat);
  let result = state.create_room(&request2, owner_id);
  assert_eq!(result.unwrap_err(), RoomError::AlreadyOwnerOfSameType);
}

#[test]
fn test_create_room_same_owner_different_type_allowed() {
  let state = RoomState::new();
  let owner_id = UserId::new();

  let request1 = create_room_request("Chat Room", RoomType::Chat);
  state.create_room(&request1, owner_id.clone()).unwrap();

  let request2 = create_room_request("Theater Room", RoomType::Theater);
  let result = state.create_room(&request2, owner_id);
  assert!(result.is_ok());
  assert_eq!(state.room_count(), 2);
}

#[test]
fn test_create_room_with_password() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let request = create_room_request_with_password("Secret Room", "pass1234");

  let result = state.create_room(&request, owner_id);
  assert!(result.is_ok());

  let room = state.get_room(&result.unwrap().0).unwrap();
  assert!(room.is_password_protected());
}

#[test]
fn test_create_room_max_members_clamped() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let request = CreateRoom {
    name: "Big Room".to_string(),
    room_type: RoomType::Chat,
    max_participants: 100, // exceeds MAX_MEMBERS_PER_ROOM
    password: None,
  };

  let result = state.create_room(&request, owner_id).unwrap();
  let room = state.get_room(&result.0).unwrap();
  assert_eq!(room.info.max_members, MAX_MEMBERS_PER_ROOM);
}

#[test]
fn test_create_room_min_members_clamped() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let request = CreateRoom {
    name: "Tiny Room".to_string(),
    room_type: RoomType::Chat,
    max_participants: 1, // below minimum of 2
    password: None,
  };

  let result = state.create_room(&request, owner_id).unwrap();
  let room = state.get_room(&result.0).unwrap();
  assert_eq!(room.info.max_members, 2);
}

// ===========================================================================
// Join room
// ===========================================================================

#[test]
fn test_join_room_success() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  let (room_id, _) = state
    .create_room(&create_room_request("Test", RoomType::Chat), owner_id)
    .unwrap();

  let request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  let result = state.join_room(&request, member_id.clone(), "Alice".to_string());
  assert!(result.is_ok());

  let (room_info, members) = result.unwrap();
  assert_eq!(members.len(), 2);
  assert_eq!(state.total_member_count(), 2);
  assert_eq!(state.get_user_room(&member_id), Some(room_id));
  // room_info is available for caller use
  let _ = room_info;
}

#[test]
fn test_join_room_not_found() {
  let state = RoomState::new();
  let member_id = UserId::new();

  let request = JoinRoom {
    room_id: RoomId::new(),
    password: None,
  };
  let result = state.join_room(&request, member_id, "Alice".to_string());
  assert_eq!(result.unwrap_err(), RoomError::RoomNotFound);
}

#[test]
fn test_join_room_already_in_room() {
  let state = RoomState::new();
  let owner_id = UserId::new();

  let (room_id, _) = state
    .create_room(
      &create_room_request("Test", RoomType::Chat),
      owner_id.clone(),
    )
    .unwrap();

  let request = JoinRoom {
    room_id,
    password: None,
  };
  let result = state.join_room(&request, owner_id, "Alice".to_string());
  assert_eq!(result.unwrap_err(), RoomError::UserAlreadyInRoom);
}

#[test]
fn test_join_room_password_protected_correct() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  let (room_id, _) = state
    .create_room(
      &create_room_request_with_password("Secret", "pass1234"),
      owner_id,
    )
    .unwrap();

  let request = JoinRoom {
    room_id,
    password: Some("pass1234".to_string()),
  };
  let result = state.join_room(&request, member_id, "Alice".to_string());
  assert!(result.is_ok());
}

#[test]
fn test_join_room_password_protected_wrong() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  let (room_id, _) = state
    .create_room(
      &create_room_request_with_password("Secret", "pass1234"),
      owner_id,
    )
    .unwrap();

  let request = JoinRoom {
    room_id,
    password: Some("wrong".to_string()),
  };
  let result = state.join_room(&request, member_id, "Alice".to_string());
  assert!(matches!(result.unwrap_err(), RoomError::InvalidPassword(_)));
}

#[test]
fn test_join_room_password_protected_none_provided() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  let (room_id, _) = state
    .create_room(
      &create_room_request_with_password("Secret", "pass1234"),
      owner_id,
    )
    .unwrap();

  let request = JoinRoom {
    room_id,
    password: None,
  };
  let result = state.join_room(&request, member_id, "Alice".to_string());
  assert!(matches!(result.unwrap_err(), RoomError::InvalidPassword(_)));
}

// ===========================================================================
// Leave room
// ===========================================================================

#[test]
fn test_leave_room_success() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  let (room_id, _) = state
    .create_room(&create_room_request("Test", RoomType::Chat), owner_id)
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Alice".to_string())
    .unwrap();

  let leave_request = LeaveRoom {
    room_id: room_id.clone(),
  };
  let result = state.leave_room(&leave_request, &member_id);
  assert!(result.is_ok());

  let leave_result = result.unwrap();
  assert!(!leave_result.room_destroyed);
  assert!(leave_result.room_info.is_some());
  assert_eq!(state.get_user_room(&member_id), None);
}

#[test]
fn test_leave_room_owner_transfers_ownership() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  let (room_id, _) = state
    .create_room(
      &create_room_request("Test", RoomType::Chat),
      owner_id.clone(),
    )
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Alice".to_string())
    .unwrap();

  // Promote member to admin
  let promote_request = PromoteAdmin {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  state.promote_admin(&promote_request, &owner_id).unwrap();

  // Owner leaves
  let leave_request = LeaveRoom {
    room_id: room_id.clone(),
  };
  let result = state.leave_room(&leave_request, &owner_id);
  assert!(result.is_ok());

  let leave_result = result.unwrap();
  assert!(leave_result.ownership_transfer.is_some());
  assert_eq!(leave_result.ownership_transfer.unwrap(), member_id);
}

#[test]
fn test_leave_room_destroys_when_empty() {
  let state = RoomState::new();
  let owner_id = UserId::new();

  let (room_id, _) = state
    .create_room(
      &create_room_request("Test", RoomType::Chat),
      owner_id.clone(),
    )
    .unwrap();

  let leave_request = LeaveRoom {
    room_id: room_id.clone(),
  };
  let result = state.leave_room(&leave_request, &owner_id);

  let leave_result = result.unwrap();
  assert!(leave_result.room_destroyed);
  assert_eq!(state.room_count(), 0);
}

#[test]
fn test_leave_room_not_in_room() {
  let state = RoomState::new();
  let user_id = UserId::new();
  let leave_request = LeaveRoom {
    room_id: RoomId::new(),
  };

  let result = state.leave_room(&leave_request, &user_id);
  assert_eq!(result.unwrap_err(), RoomError::UserNotInRoom);
}

// ===========================================================================
// Permission checks
// ===========================================================================

#[test]
fn test_check_permission_owner_can_kick_member() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  let (room_id, _) = state
    .create_room(
      &create_room_request("Test", RoomType::Chat),
      owner_id.clone(),
    )
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Alice".to_string())
    .unwrap();

  let result = state.check_permission(&room_id, &owner_id, &member_id, ModerationAction::Kicked);
  assert!(result.is_some());
  assert!(result.unwrap().can_act);
}

#[test]
fn test_check_permission_member_cannot_kick_member() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let member1 = UserId::new();
  let member2 = UserId::new();

  let (room_id, _) = state
    .create_room(&create_room_request("Test", RoomType::Chat), owner_id)
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request.clone(), member1.clone(), "A".to_string())
    .unwrap();
  state
    .join_room(&join_request, member2.clone(), "B".to_string())
    .unwrap();

  let result = state.check_permission(&room_id, &member1, &member2, ModerationAction::Kicked);
  assert!(result.is_some());
  assert!(!result.unwrap().can_act);
}

#[test]
fn test_check_permission_room_not_found() {
  let state = RoomState::new();
  let user = UserId::new();

  let result = state.check_permission(&RoomId::new(), &user, &user, ModerationAction::Kicked);
  assert!(result.is_none());
}

// ===========================================================================
// Kick member
// ===========================================================================

#[test]
fn test_kick_member_success() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  let (room_id, _) = state
    .create_room(
      &create_room_request("Test", RoomType::Chat),
      owner_id.clone(),
    )
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Alice".to_string())
    .unwrap();

  let kick_request = KickMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = state.kick_member(&kick_request, &owner_id);
  assert!(result.is_ok());

  let (removed_member, room_info) = result.unwrap();
  assert_eq!(removed_member.user_id, member_id);
  assert_eq!(room_info.member_count, 1);
  assert_eq!(state.get_user_room(&member_id), None);
}

#[test]
fn test_kick_member_insufficient_permission() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let member1 = UserId::new();
  let member2 = UserId::new();

  let (room_id, _) = state
    .create_room(&create_room_request("Test", RoomType::Chat), owner_id)
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request.clone(), member1.clone(), "A".to_string())
    .unwrap();
  state
    .join_room(&join_request, member2.clone(), "B".to_string())
    .unwrap();

  let kick_request = KickMember {
    room_id: room_id.clone(),
    target: member2,
  };
  let result = state.kick_member(&kick_request, &member1);
  assert_eq!(result.unwrap_err(), RoomError::InsufficientPermission);
}

// ===========================================================================
// Mute / Unmute
// ===========================================================================

#[test]
fn test_mute_member_success() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  let (room_id, _) = state
    .create_room(
      &create_room_request("Test", RoomType::Chat),
      owner_id.clone(),
    )
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Alice".to_string())
    .unwrap();

  let mute_request = MuteMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
    duration_secs: Some(300),
  };
  let result = state.mute_member(&mute_request, &owner_id);
  assert!(result.is_ok());

  let (member_info, mute_info) = result.unwrap();
  assert_eq!(member_info.user_id, member_id);
  assert!(matches!(mute_info, MuteInfo::Timed { .. }));
}

#[test]
fn test_unmute_member_success() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  let (room_id, _) = state
    .create_room(
      &create_room_request("Test", RoomType::Chat),
      owner_id.clone(),
    )
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Alice".to_string())
    .unwrap();

  let mute_request = MuteMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
    duration_secs: None,
  };
  state.mute_member(&mute_request, &owner_id).unwrap();

  let unmute_request = UnmuteMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = state.unmute_member(&unmute_request, &owner_id);
  assert!(result.is_ok());
}

// ===========================================================================
// Ban / Unban
// ===========================================================================

#[test]
fn test_ban_member_success() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  let (room_id, _) = state
    .create_room(
      &create_room_request("Test", RoomType::Chat),
      owner_id.clone(),
    )
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Alice".to_string())
    .unwrap();

  let ban_request = BanMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = state.ban_member(&ban_request, &owner_id);
  assert!(result.is_ok());

  let (_, room_info) = result.unwrap();
  assert_eq!(room_info.member_count, 1);
  assert_eq!(state.get_user_room(&member_id), None);

  // Banned user should not be able to rejoin
  let room = state.get_room(&room_id).unwrap();
  assert!(room.is_banned(&member_id));
}

#[test]
fn test_unban_member_success() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  let (room_id, _) = state
    .create_room(
      &create_room_request("Test", RoomType::Chat),
      owner_id.clone(),
    )
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Alice".to_string())
    .unwrap();

  let ban_request = BanMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  state.ban_member(&ban_request, &owner_id).unwrap();

  let unban_request = UnbanMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = state.unban_member(&unban_request, &owner_id);
  assert!(result.is_ok());

  let room = state.get_room(&room_id).unwrap();
  assert!(!room.is_banned(&member_id));
}

// ===========================================================================
// Promote / Demote admin
// ===========================================================================

#[test]
fn test_promote_admin_success() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  let (room_id, _) = state
    .create_room(
      &create_room_request("Test", RoomType::Chat),
      owner_id.clone(),
    )
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Alice".to_string())
    .unwrap();

  let promote_request = PromoteAdmin {
    room_id,
    target: member_id.clone(),
  };
  let result = state.promote_admin(&promote_request, &owner_id);
  assert!(result.is_ok());
  assert_eq!(result.unwrap().role, RoomRole::Admin);
}

#[test]
fn test_demote_admin_success() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  let (room_id, _) = state
    .create_room(
      &create_room_request("Test", RoomType::Chat),
      owner_id.clone(),
    )
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Alice".to_string())
    .unwrap();

  let promote_request = PromoteAdmin {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  state.promote_admin(&promote_request, &owner_id).unwrap();

  let demote_request = DemoteAdmin {
    room_id,
    target: member_id.clone(),
  };
  let result = state.demote_admin(&demote_request, &owner_id);
  assert!(result.is_ok());
  assert_eq!(result.unwrap().role, RoomRole::Member);
}

// ===========================================================================
// Transfer ownership
// ===========================================================================

#[test]
fn test_transfer_ownership_success() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  let (room_id, _) = state
    .create_room(
      &create_room_request("Test", RoomType::Chat),
      owner_id.clone(),
    )
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Alice".to_string())
    .unwrap();

  let transfer_request = TransferOwnership {
    room_id,
    target: member_id.clone(),
  };
  let result = state.transfer_ownership(&transfer_request, &owner_id);
  assert!(result.is_ok());

  let (old_owner, new_owner) = result.unwrap();
  assert_eq!(old_owner.role, RoomRole::Admin);
  assert_eq!(new_owner.role, RoomRole::Owner);
}

#[test]
fn test_transfer_ownership_non_owner_rejected() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  let (room_id, _) = state
    .create_room(&create_room_request("Test", RoomType::Chat), owner_id)
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Alice".to_string())
    .unwrap();

  let transfer_request = TransferOwnership {
    room_id,
    target: member_id,
  };
  // member cannot transfer ownership
  let result = state.transfer_ownership(&transfer_request, &UserId::new());
  assert_eq!(result.unwrap_err(), RoomError::InsufficientPermission);
}

// ===========================================================================
// Announcement
// ===========================================================================

#[test]
fn test_set_announcement_by_owner() {
  let state = RoomState::new();
  let owner_id = UserId::new();

  let (room_id, _) = state
    .create_room(
      &create_room_request("Test", RoomType::Chat),
      owner_id.clone(),
    )
    .unwrap();

  let announcement = RoomAnnouncement {
    room_id,
    content: "Welcome!".to_string(),
  };
  let result = state.set_announcement(&announcement, &owner_id);
  assert!(result.is_ok());
}

#[test]
fn test_set_announcement_non_owner_rejected() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  let (room_id, _) = state
    .create_room(&create_room_request("Test", RoomType::Chat), owner_id)
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Alice".to_string())
    .unwrap();

  let announcement = RoomAnnouncement {
    room_id,
    content: "Hacked!".to_string(),
  };
  let result = state.set_announcement(&announcement, &member_id);
  assert_eq!(result.unwrap_err(), RoomError::InsufficientPermission);
}

// ===========================================================================
// Nickname
// ===========================================================================

#[test]
fn test_set_nickname_success() {
  let state = RoomState::new();
  let owner_id = UserId::new();

  let (room_id, _) = state
    .create_room(
      &create_room_request("Test", RoomType::Chat),
      owner_id.clone(),
    )
    .unwrap();

  let request = NicknameChange {
    user_id: owner_id.clone(),
    new_nickname: "NewName".to_string(),
  };
  let result = state.set_nickname(&request);
  assert!(result.is_ok());

  let room = state.get_room(&room_id).unwrap();
  assert_eq!(room.get_member(&owner_id).unwrap().nickname, "NewName");
}

// ===========================================================================
// Remove user from all rooms (disconnect)
// ===========================================================================

#[test]
fn test_remove_user_from_all_rooms() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  let (room_id, _) = state
    .create_room(&create_room_request("Test", RoomType::Chat), owner_id)
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Alice".to_string())
    .unwrap();

  let results = state.remove_user_from_all_rooms(&member_id);
  assert_eq!(results.len(), 1);
  assert_eq!(state.get_user_room(&member_id), None);
}

// ===========================================================================
// Utility methods
// ===========================================================================

#[test]
fn test_get_room_found() {
  let state = RoomState::new();
  let owner_id = UserId::new();

  let (room_id, _) = state
    .create_room(&create_room_request("Test", RoomType::Chat), owner_id)
    .unwrap();

  assert!(state.get_room(&room_id).is_some());
}

#[test]
fn test_get_room_not_found() {
  let state = RoomState::new();
  assert!(state.get_room(&RoomId::new()).is_none());
}

#[test]
fn test_get_all_rooms() {
  let state = RoomState::new();
  let owner1 = UserId::new();
  let owner2 = UserId::new();

  state
    .create_room(&create_room_request("Room1", RoomType::Chat), owner1)
    .unwrap();
  state
    .create_room(&create_room_request("Room2", RoomType::Theater), owner2)
    .unwrap();

  let rooms = state.get_all_rooms();
  assert_eq!(rooms.len(), 2);
}

#[test]
fn test_get_room_members() {
  let state = RoomState::new();
  let owner_id = UserId::new();

  let (room_id, _) = state
    .create_room(&create_room_request("Test", RoomType::Chat), owner_id)
    .unwrap();

  let members = state.get_room_members(&room_id);
  assert!(members.is_some());
  assert_eq!(members.unwrap().len(), 1);
}

#[test]
fn test_room_count() {
  let state = RoomState::new();
  let owner1 = UserId::new();
  let owner2 = UserId::new();

  state
    .create_room(&create_room_request("Room1", RoomType::Chat), owner1)
    .unwrap();
  assert_eq!(state.room_count(), 1);

  state
    .create_room(&create_room_request("Room2", RoomType::Theater), owner2)
    .unwrap();
  assert_eq!(state.room_count(), 2);
}

#[test]
fn test_total_member_count() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  let (room_id, _) = state
    .create_room(&create_room_request("Test", RoomType::Chat), owner_id)
    .unwrap();
  assert_eq!(state.total_member_count(), 1);

  let join_request = JoinRoom {
    room_id,
    password: None,
  };
  state
    .join_room(&join_request, member_id, "Alice".to_string())
    .unwrap();
  assert_eq!(state.total_member_count(), 2);
}

// ===========================================================================
// get_user_room edge cases
// ===========================================================================

#[test]
fn test_get_user_room_not_in_any_room() {
  let state = RoomState::new();
  let unknown_user = UserId::new();

  // User not in any room should return None
  assert_eq!(state.get_user_room(&unknown_user), None);
}

#[test]
fn test_get_user_room_after_leaving() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  let (room_id, _) = state
    .create_room(&create_room_request("Test", RoomType::Chat), owner_id)
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Alice".to_string())
    .unwrap();

  // Member is in room
  assert_eq!(state.get_user_room(&member_id), Some(room_id.clone()));

  // Member leaves
  let leave_request = LeaveRoom {
    room_id: room_id.clone(),
  };
  state.leave_room(&leave_request, &member_id).unwrap();

  // After leaving, user should not be in any room
  assert_eq!(state.get_user_room(&member_id), None);
}

// ===========================================================================
// get_room_members edge cases
// ===========================================================================

#[test]
fn test_get_room_members_nonexistent_room() {
  let state = RoomState::new();
  let fake_room_id = RoomId::new();

  // Non-existent room should return None
  assert_eq!(state.get_room_members(&fake_room_id), None);
}

#[test]
fn test_get_room_members_multiple_members() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let member1_id = UserId::new();
  let member2_id = UserId::new();

  let (room_id, _) = state
    .create_room(&create_room_request("Test", RoomType::Chat), owner_id)
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request.clone(), member1_id, "Alice".to_string())
    .unwrap();
  state
    .join_room(&join_request, member2_id, "Bob".to_string())
    .unwrap();

  let members = state.get_room_members(&room_id).unwrap();
  assert_eq!(members.len(), 3); // owner + 2 members
}

// ===========================================================================
// check_expired_mutes
// ===========================================================================

#[test]
fn test_check_expired_mutes_no_rooms() {
  let state = RoomState::new();

  // No rooms, should return empty map
  let expired = state.check_expired_mutes();
  assert!(expired.is_empty());
}

#[test]
fn test_check_expired_mutes_no_mutes() {
  let state = RoomState::new();
  let owner_id = UserId::new();

  state
    .create_room(&create_room_request("Test", RoomType::Chat), owner_id)
    .unwrap();

  // Room with no mutes, should return empty map
  let expired = state.check_expired_mutes();
  assert!(expired.is_empty());
}

#[test]
fn test_check_expired_mutes_active_mutes() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  let (room_id, _) = state
    .create_room(
      &create_room_request("Test", RoomType::Chat),
      owner_id.clone(),
    )
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Alice".to_string())
    .unwrap();

  // Mute with long duration (should not expire)
  let mute_request = MuteMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
    duration_secs: Some(3600), // 1 hour
  };
  state.mute_member(&mute_request, &owner_id).unwrap();

  // Check immediately - no expired mutes
  let expired = state.check_expired_mutes();
  assert!(expired.is_empty());
}

#[test]
fn test_check_expired_mutes_expired_mutes() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  let (room_id, _) = state
    .create_room(
      &create_room_request("Test", RoomType::Chat),
      owner_id.clone(),
    )
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Alice".to_string())
    .unwrap();

  // Mute with very short duration (1 second)
  let mute_request = MuteMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
    duration_secs: Some(1),
  };
  state.mute_member(&mute_request, &owner_id).unwrap();

  // Wait for mute to expire
  std::thread::sleep(std::time::Duration::from_millis(1500));

  // Check - should find expired mute
  let expired = state.check_expired_mutes();
  assert_eq!(expired.len(), 1);
  assert!(expired.contains_key(&room_id));
  assert!(expired[&room_id].contains(&member_id));

  // Member should no longer be muted
  let room = state.get_room(&room_id).unwrap();
  assert!(!room.get_member(&member_id).unwrap().is_muted());
}

#[test]
fn test_check_expired_mutes_multiple_rooms() {
  let state = RoomState::new();
  let owner1_id = UserId::new();
  let owner2_id = UserId::new();
  let member1_id = UserId::new();
  let member2_id = UserId::new();

  // Create two rooms
  let (room1_id, _) = state
    .create_room(
      &create_room_request("Room1", RoomType::Chat),
      owner1_id.clone(),
    )
    .unwrap();
  let (room2_id, _) = state
    .create_room(
      &create_room_request("Room2", RoomType::Theater),
      owner2_id.clone(),
    )
    .unwrap();

  // Add members
  state
    .join_room(
      &JoinRoom {
        room_id: room1_id.clone(),
        password: None,
      },
      member1_id.clone(),
      "Alice".to_string(),
    )
    .unwrap();
  state
    .join_room(
      &JoinRoom {
        room_id: room2_id.clone(),
        password: None,
      },
      member2_id.clone(),
      "Bob".to_string(),
    )
    .unwrap();

  // Mute both with short duration
  state
    .mute_member(
      &MuteMember {
        room_id: room1_id.clone(),
        target: member1_id.clone(),
        duration_secs: Some(1),
      },
      &owner1_id,
    )
    .unwrap();
  state
    .mute_member(
      &MuteMember {
        room_id: room2_id.clone(),
        target: member2_id.clone(),
        duration_secs: Some(1),
      },
      &owner2_id,
    )
    .unwrap();

  // Wait for mutes to expire
  std::thread::sleep(std::time::Duration::from_millis(1500));

  // Check - should find expired mutes in both rooms
  let expired = state.check_expired_mutes();
  assert_eq!(expired.len(), 2);
  assert!(expired.contains_key(&room1_id));
  assert!(expired.contains_key(&room2_id));
}

#[test]
fn test_check_expired_mutes_permanent_mute_stays() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  let (room_id, _) = state
    .create_room(
      &create_room_request("Test", RoomType::Chat),
      owner_id.clone(),
    )
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Alice".to_string())
    .unwrap();

  // Permanent mute (None duration)
  let mute_request = MuteMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
    duration_secs: None, // permanent
  };
  state.mute_member(&mute_request, &owner_id).unwrap();

  // Wait some time
  std::thread::sleep(std::time::Duration::from_millis(500));

  // Check - permanent mute should not expire
  let expired = state.check_expired_mutes();
  assert!(expired.is_empty());

  // Member should still be muted
  let room = state.get_room(&room_id).unwrap();
  assert!(room.get_member(&member_id).unwrap().is_muted());
}

// ===========================================================================
// Additional edge cases for room statistics
// ===========================================================================

#[test]
fn test_room_count_after_destruction() {
  let state = RoomState::new();
  let owner_id = UserId::new();

  let (room_id, _) = state
    .create_room(
      &create_room_request("Test", RoomType::Chat),
      owner_id.clone(),
    )
    .unwrap();
  assert_eq!(state.room_count(), 1);

  // Owner leaves - room should be destroyed (no other members)
  let leave_request = LeaveRoom { room_id };
  state.leave_room(&leave_request, &owner_id).unwrap();

  assert_eq!(state.room_count(), 0);
}

#[test]
fn test_total_member_count_after_room_destruction() {
  let state = RoomState::new();
  let owner_id = UserId::new();

  let (room_id, _) = state
    .create_room(
      &create_room_request("Test", RoomType::Chat),
      owner_id.clone(),
    )
    .unwrap();
  assert_eq!(state.total_member_count(), 1);

  // Owner leaves - room destroyed, member count should be 0
  let leave_request = LeaveRoom { room_id };
  state.leave_room(&leave_request, &owner_id).unwrap();

  assert_eq!(state.total_member_count(), 0);
}

// =============================================================================
// Room Name Validation Tests
// =============================================================================

#[test]
fn test_create_room_with_leading_whitespace_rejected() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let request = CreateRoom {
    name: " Test Room".to_string(),
    room_type: RoomType::Chat,
    max_participants: 8,
    password: None,
  };

  let result = state.create_room(&request, owner_id);
  assert!(result.is_err());
  match result.unwrap_err() {
    RoomError::InvalidRoomName(msg) => assert!(msg.contains("whitespace")),
    _ => panic!("Expected InvalidRoomName error"),
  }
}

#[test]
fn test_create_room_with_trailing_whitespace_rejected() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let request = CreateRoom {
    name: "Test Room ".to_string(),
    room_type: RoomType::Chat,
    max_participants: 8,
    password: None,
  };

  let result = state.create_room(&request, owner_id);
  assert!(result.is_err());
  match result.unwrap_err() {
    RoomError::InvalidRoomName(msg) => assert!(msg.contains("whitespace")),
    _ => panic!("Expected InvalidRoomName error"),
  }
}

#[test]
fn test_create_room_with_consecutive_spaces_rejected() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let request = CreateRoom {
    name: "Test  Room".to_string(),
    room_type: RoomType::Chat,
    max_participants: 8,
    password: None,
  };

  let result = state.create_room(&request, owner_id);
  assert!(result.is_err());
  match result.unwrap_err() {
    RoomError::InvalidRoomName(msg) => assert!(msg.contains("consecutive")),
    _ => panic!("Expected InvalidRoomName error"),
  }
}

#[test]
fn test_create_room_with_special_characters_rejected() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let request = CreateRoom {
    name: "Test@Room".to_string(),
    room_type: RoomType::Chat,
    max_participants: 8,
    password: None,
  };

  let result = state.create_room(&request, owner_id);
  assert!(result.is_err());
  match result.unwrap_err() {
    RoomError::InvalidRoomName(msg) => assert!(msg.contains("invalid character")),
    _ => panic!("Expected InvalidRoomName error"),
  }
}

#[test]
fn test_create_room_with_emoji_rejected() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let request = CreateRoom {
    name: "Test 🎉 Room".to_string(),
    room_type: RoomType::Chat,
    max_participants: 8,
    password: None,
  };

  let result = state.create_room(&request, owner_id);
  assert!(result.is_err());
  match result.unwrap_err() {
    RoomError::InvalidRoomName(msg) => assert!(msg.contains("invalid character")),
    _ => panic!("Expected InvalidRoomName error"),
  }
}

#[test]
fn test_create_room_with_valid_characters_allowed() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let request = CreateRoom {
    name: "Test-Room_123 聊天室".to_string(),
    room_type: RoomType::Chat,
    max_participants: 8,
    password: None,
  };

  let result = state.create_room(&request, owner_id);
  assert!(result.is_ok());
  let (_, info) = result.unwrap();
  assert_eq!(info.name, "Test-Room_123 聊天室");
}

#[test]
fn test_create_room_with_middle_dot_allowed() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let request = CreateRoom {
    name: "Test·Room".to_string(),
    room_type: RoomType::Chat,
    max_participants: 8,
    password: None,
  };

  let result = state.create_room(&request, owner_id);
  assert!(result.is_ok());
}

#[test]
fn test_create_room_with_sql_injection_attempt_rejected() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let request = CreateRoom {
    name: "Room'; DROP TABLE rooms;--".to_string(),
    room_type: RoomType::Chat,
    max_participants: 8,
    password: None,
  };

  let result = state.create_room(&request, owner_id);
  assert!(result.is_err());
  match result.unwrap_err() {
    RoomError::InvalidRoomName(msg) => assert!(msg.contains("invalid character")),
    _ => panic!("Expected InvalidRoomName error"),
  }
}

#[test]
fn test_create_room_with_html_injection_attempt_rejected() {
  let state = RoomState::new();
  let owner_id = UserId::new();
  let request = CreateRoom {
    name: "<script>alert('xss')</script>".to_string(),
    room_type: RoomType::Chat,
    max_participants: 8,
    password: None,
  };

  let result = state.create_room(&request, owner_id);
  assert!(result.is_err());
  match result.unwrap_err() {
    RoomError::InvalidRoomName(msg) => assert!(msg.contains("invalid character")),
    _ => panic!("Expected InvalidRoomName error"),
  }
}
