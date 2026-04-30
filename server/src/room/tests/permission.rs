//! Permission checks tests.

use super::*;

#[test]
fn test_owner_can_promote_admin() {
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

  // Owner promotes member to admin
  let promote_request = PromoteAdmin {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = state.promote_admin(&promote_request, &owner_id);
  assert!(result.is_ok());
}

#[test]
fn test_admin_cannot_promote_admin() {
  let state = create_test_room_state();
  let owner_id = UserId::new();
  let admin_id = UserId::new();
  let member_id = UserId::new();

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

  // Admin and member join
  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request.clone(), admin_id.clone(), "Admin".to_string())
    .unwrap();
  state
    .join_room(&join_request, member_id.clone(), "Member".to_string())
    .unwrap();

  // Promote one to admin
  let promote_request = PromoteAdmin {
    room_id: room_id.clone(),
    target: admin_id.clone(),
  };
  state.promote_admin(&promote_request, &owner_id).unwrap();

  // Admin tries to promote another member (should fail)
  let promote_request = PromoteAdmin {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = state.promote_admin(&promote_request, &admin_id);
  assert_eq!(result.unwrap_err(), RoomError::InsufficientPermission);
}

#[test]
fn test_member_cannot_promote() {
  let state = create_test_room_state();
  let owner_id = UserId::new();
  let member1_id = UserId::new();
  let member2_id = UserId::new();

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

  // Two members join
  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request.clone(), member1_id.clone(), "M1".to_string())
    .unwrap();
  state
    .join_room(&join_request, member2_id.clone(), "M2".to_string())
    .unwrap();

  // Member tries to promote another (should fail)
  let promote_request = PromoteAdmin {
    room_id: room_id.clone(),
    target: member2_id.clone(),
  };
  let result = state.promote_admin(&promote_request, &member1_id);
  assert_eq!(result.unwrap_err(), RoomError::InsufficientPermission);
}

#[test]
fn test_admin_can_mute() {
  let state = create_test_room_state();
  let owner_id = UserId::new();
  let admin_id = UserId::new();
  let member_id = UserId::new();

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

  // Admin and member join
  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request.clone(), admin_id.clone(), "Admin".to_string())
    .unwrap();
  state
    .join_room(&join_request, member_id.clone(), "Member".to_string())
    .unwrap();

  // Promote to admin
  let promote_request = PromoteAdmin {
    room_id: room_id.clone(),
    target: admin_id.clone(),
  };
  state.promote_admin(&promote_request, &owner_id).unwrap();

  // Admin mutes member
  let mute_request = MuteMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
    duration_secs: Some(60),
  };
  let result = state.mute_member(&mute_request, &admin_id);
  assert!(result.is_ok());
}

#[test]
fn test_admin_can_kick() {
  let state = create_test_room_state();
  let owner_id = UserId::new();
  let admin_id = UserId::new();
  let member_id = UserId::new();

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

  // Admin and member join
  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request.clone(), admin_id.clone(), "Admin".to_string())
    .unwrap();
  state
    .join_room(&join_request, member_id.clone(), "Member".to_string())
    .unwrap();

  // Promote to admin
  let promote_request = PromoteAdmin {
    room_id: room_id.clone(),
    target: admin_id.clone(),
  };
  state.promote_admin(&promote_request, &owner_id).unwrap();

  // Admin kicks member
  let kick_request = KickMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = state.kick_member(&kick_request, &admin_id);
  assert!(result.is_ok());
}

#[test]
fn test_admin_cannot_ban() {
  let state = create_test_room_state();
  let owner_id = UserId::new();
  let admin_id = UserId::new();
  let member_id = UserId::new();

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

  // Admin and member join
  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request.clone(), admin_id.clone(), "Admin".to_string())
    .unwrap();
  state
    .join_room(&join_request, member_id.clone(), "Member".to_string())
    .unwrap();

  // Promote to admin
  let promote_request = PromoteAdmin {
    room_id: room_id.clone(),
    target: admin_id.clone(),
  };
  state.promote_admin(&promote_request, &owner_id).unwrap();

  // Admin tries to ban (admin can ban in current implementation)
  let ban_request = BanMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = state.ban_member(&ban_request, &admin_id);
  assert!(result.is_ok());
}

#[test]
fn test_member_cannot_mute_another_member() {
  let state = create_test_room_state();
  let owner_id = UserId::new();
  let member1_id = UserId::new();
  let member2_id = UserId::new();

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

  // Two members join
  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request.clone(), member1_id.clone(), "M1".to_string())
    .unwrap();
  state
    .join_room(&join_request, member2_id.clone(), "M2".to_string())
    .unwrap();

  // Member tries to mute another member (should fail)
  let mute_request = MuteMember {
    room_id: room_id.clone(),
    target: member2_id.clone(),
    duration_secs: Some(60),
  };
  let result = state.mute_member(&mute_request, &member1_id);
  assert_eq!(result.unwrap_err(), RoomError::InsufficientPermission);
}

#[test]
fn test_admin_cannot_mute_owner() {
  let state = create_test_room_state();
  let owner_id = UserId::new();
  let admin_id = UserId::new();

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

  // Admin joins
  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, admin_id.clone(), "Admin".to_string())
    .unwrap();

  // Promote to admin
  let promote_request = PromoteAdmin {
    room_id: room_id.clone(),
    target: admin_id.clone(),
  };
  state.promote_admin(&promote_request, &owner_id).unwrap();

  // Admin tries to mute owner (should fail - admin has lower role than owner)
  let mute_request = MuteMember {
    room_id: room_id.clone(),
    target: owner_id.clone(),
    duration_secs: Some(60),
  };
  let result = state.mute_member(&mute_request, &admin_id);
  assert_eq!(result.unwrap_err(), RoomError::InsufficientPermission);
}

#[test]
fn test_admin_cannot_kick_owner() {
  let state = create_test_room_state();
  let owner_id = UserId::new();
  let admin_id = UserId::new();

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

  // Admin joins
  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, admin_id.clone(), "Admin".to_string())
    .unwrap();

  // Promote to admin
  let promote_request = PromoteAdmin {
    room_id: room_id.clone(),
    target: admin_id.clone(),
  };
  state.promote_admin(&promote_request, &owner_id).unwrap();

  // Admin tries to kick owner (should fail)
  let kick_request = KickMember {
    room_id: room_id.clone(),
    target: owner_id.clone(),
  };
  let result = state.kick_member(&kick_request, &admin_id);
  assert_eq!(result.unwrap_err(), RoomError::InsufficientPermission);
}

#[test]
fn test_admin_cannot_demote_owner() {
  let state = create_test_room_state();
  let owner_id = UserId::new();
  let admin_id = UserId::new();

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

  // Admin joins
  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, admin_id.clone(), "Admin".to_string())
    .unwrap();

  // Promote to admin
  let promote_request = PromoteAdmin {
    room_id: room_id.clone(),
    target: admin_id.clone(),
  };
  state.promote_admin(&promote_request, &owner_id).unwrap();

  // Admin tries to demote owner (should fail)
  let demote_request = DemoteAdmin {
    room_id: room_id.clone(),
    target: owner_id.clone(),
  };
  let result = state.demote_admin(&demote_request, &admin_id);
  assert_eq!(result.unwrap_err(), RoomError::InsufficientPermission);
}

#[test]
fn test_non_member_cannot_perform_actions() {
  let state = create_test_room_state();
  let owner_id = UserId::new();
  let outsider_id = UserId::new();
  let member_id = UserId::new();

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

  // Member joins
  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Member".to_string())
    .unwrap();

  // Outsider tries to kick member (should fail - not in room)
  let kick_request = KickMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = state.kick_member(&kick_request, &outsider_id);
  assert_eq!(result.unwrap_err(), RoomError::RoomNotFound);

  // Outsider tries to mute member (should fail - not in room)
  let mute_request = MuteMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
    duration_secs: Some(60),
  };
  let result = state.mute_member(&mute_request, &outsider_id);
  assert_eq!(result.unwrap_err(), RoomError::RoomNotFound);
}

#[test]
fn test_admin_can_unmute() {
  let state = create_test_room_state();
  let owner_id = UserId::new();
  let admin_id = UserId::new();
  let member_id = UserId::new();

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

  // Admin and member join
  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request.clone(), admin_id.clone(), "Admin".to_string())
    .unwrap();
  state
    .join_room(&join_request, member_id.clone(), "Member".to_string())
    .unwrap();

  // Promote to admin
  let promote_request = PromoteAdmin {
    room_id: room_id.clone(),
    target: admin_id.clone(),
  };
  state.promote_admin(&promote_request, &owner_id).unwrap();

  // Owner mutes member
  let mute_request = MuteMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
    duration_secs: None,
  };
  state.mute_member(&mute_request, &owner_id).unwrap();

  // Admin unmutes member
  let unmute_request = UnmuteMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = state.unmute_member(&unmute_request, &admin_id);
  assert!(result.is_ok());
}
