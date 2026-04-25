//! Moderation tests: permission checks, kick, mute, ban, announcement.

use message::signaling::{
  BanMember, KickMember, MuteMember, RoomAnnouncement, UnbanMember, UnmuteMember,
};
use message::types::RoomRole;

use super::super::RoomState;
use super::*;

// ===========================================================================
// Permission Checks
// ===========================================================================

#[test]
fn test_check_permission_owner_can_kick_admin() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let admin_id = test_user_id(2);
  setup_room_with_admin(&state, &owner_id, &admin_id);

  let room_id = state.get_user_room(&owner_id).unwrap();
  let result = state.check_permission(
    &room_id,
    &owner_id,
    &admin_id,
    message::signaling::ModerationAction::Kicked,
  );

  assert!(result.is_some());
  let perm = result.unwrap();
  assert!(perm.can_act);
  assert_eq!(perm.actor_role, RoomRole::Owner);
  assert_eq!(perm.target_role, RoomRole::Admin);
}

#[test]
fn test_check_permission_admin_cannot_kick_owner() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let admin_id = test_user_id(2);
  setup_room_with_admin(&state, &owner_id, &admin_id);

  let room_id = state.get_user_room(&owner_id).unwrap();
  let result = state.check_permission(
    &room_id,
    &admin_id,
    &owner_id,
    message::signaling::ModerationAction::Kicked,
  );

  assert!(result.is_some());
  let perm = result.unwrap();
  assert!(!perm.can_act);
}

#[test]
fn test_check_permission_admin_can_kick_member() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let admin_id = test_user_id(2);
  let member_id = test_user_id(3);
  setup_room_with_admin_and_member(&state, &owner_id, &admin_id, &member_id);

  let room_id = state.get_user_room(&admin_id).unwrap();
  let result = state.check_permission(
    &room_id,
    &admin_id,
    &member_id,
    message::signaling::ModerationAction::Kicked,
  );

  assert!(result.is_some());
  let perm = result.unwrap();
  assert!(perm.can_act);
}

#[test]
fn test_check_permission_member_cannot_kick() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let member1_id = test_user_id(2);
  let member2_id = test_user_id(3);
  setup_room_with_two_members(&state, &owner_id, &member1_id, &member2_id);

  let room_id = state.get_user_room(&member1_id).unwrap();
  let result = state.check_permission(
    &room_id,
    &member1_id,
    &member2_id,
    message::signaling::ModerationAction::Kicked,
  );

  assert!(result.is_some());
  let perm = result.unwrap();
  assert!(!perm.can_act);
}

// ===========================================================================
// Kick Member
// ===========================================================================

#[test]
fn test_kick_member() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let member_id = test_user_id(2);
  let request = create_room_request();

  let (room_id, _) = state.create_room(&request, owner_id.clone()).unwrap();

  let join_request = message::signaling::JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "member".to_string())
    .unwrap();

  let kick_request = KickMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = state.kick_member(&kick_request, &owner_id);
  assert!(result.is_ok());

  let (_, room_info) = result.unwrap();
  assert_eq!(room_info.member_count, 1);

  // Kicked member should no longer be in the room
  assert!(state.get_user_room(&member_id).is_none());
}

#[test]
fn test_kick_member_insufficient_permission() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let member1_id = test_user_id(2);
  let member2_id = test_user_id(3);
  let request = create_room_request();

  let (room_id, _) = state.create_room(&request, owner_id.clone()).unwrap();

  let join1 = message::signaling::JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join1, member1_id.clone(), "m1".to_string())
    .unwrap();

  let join2 = message::signaling::JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join2, member2_id.clone(), "m2".to_string())
    .unwrap();

  // Member cannot kick another member
  let kick_request = KickMember {
    room_id: room_id.clone(),
    target: member2_id,
  };
  let result = state.kick_member(&kick_request, &member1_id);
  assert!(result.is_err());
}

// ===========================================================================
// Mute / Unmute
// ===========================================================================

#[test]
fn test_mute_member_permanent() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let member_id = test_user_id(2);
  let request = create_room_request();

  let (room_id, _) = state.create_room(&request, owner_id.clone()).unwrap();

  let join_request = message::signaling::JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "member".to_string())
    .unwrap();

  let mute_request = MuteMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
    duration_secs: None, // Permanent mute
  };
  let result = state.mute_member(&mute_request, &owner_id);
  assert!(result.is_ok());

  let (_, mute_info) = result.unwrap();
  assert!(mute_info.is_muted());
}

#[test]
fn test_mute_member_timed() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let member_id = test_user_id(2);
  let request = create_room_request();

  let (room_id, _) = state.create_room(&request, owner_id.clone()).unwrap();

  let join_request = message::signaling::JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "member".to_string())
    .unwrap();

  let mute_request = MuteMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
    duration_secs: Some(300), // 5 minute mute
  };
  let result = state.mute_member(&mute_request, &owner_id);
  assert!(result.is_ok());

  let (member_info, mute_info) = result.unwrap();
  assert!(mute_info.is_muted());
  assert!(member_info.is_muted());
}

#[test]
fn test_unmute_member() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let member_id = test_user_id(2);
  let request = create_room_request();

  let (room_id, _) = state.create_room(&request, owner_id.clone()).unwrap();

  let join_request = message::signaling::JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "member".to_string())
    .unwrap();

  // Mute first
  let mute_request = MuteMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
    duration_secs: None,
  };
  state.mute_member(&mute_request, &owner_id).unwrap();

  // Unmute
  let unmute_request = UnmuteMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = state.unmute_member(&unmute_request, &owner_id);
  assert!(result.is_ok());

  let member_info = result.unwrap();
  assert!(!member_info.is_muted());
}

#[test]
fn test_mute_member_insufficient_permission() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let member1_id = test_user_id(2);
  let member2_id = test_user_id(3);
  let request = create_room_request();

  let (room_id, _) = state.create_room(&request, owner_id.clone()).unwrap();

  let join1 = message::signaling::JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join1, member1_id.clone(), "m1".to_string())
    .unwrap();

  let join2 = message::signaling::JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join2, member2_id.clone(), "m2".to_string())
    .unwrap();

  // Member cannot mute another member
  let mute_request = MuteMember {
    room_id: room_id.clone(),
    target: member2_id,
    duration_secs: None,
  };
  let result = state.mute_member(&mute_request, &member1_id);
  assert!(result.is_err());
}

// ===========================================================================
// Ban / Unban
// ===========================================================================

#[test]
fn test_ban_member() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let member_id = test_user_id(2);
  let request = create_room_request();

  let (room_id, _) = state.create_room(&request, owner_id.clone()).unwrap();

  let join_request = message::signaling::JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "member".to_string())
    .unwrap();

  let ban_request = BanMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = state.ban_member(&ban_request, &owner_id);
  assert!(result.is_ok());

  // Banned member should no longer be in room
  assert!(state.get_user_room(&member_id).is_none());
}

#[test]
fn test_banned_user_cannot_rejoin() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let member_id = test_user_id(2);
  let request = create_room_request();

  let (room_id, _) = state.create_room(&request, owner_id.clone()).unwrap();

  let join_request = message::signaling::JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "member".to_string())
    .unwrap();

  // Ban
  let ban_request = BanMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  state.ban_member(&ban_request, &owner_id).unwrap();

  // Try to rejoin
  let rejoin_request = message::signaling::JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  let result = state.join_room(&rejoin_request, member_id, "member".to_string());
  assert!(result.is_err());
}

#[test]
fn test_unban_member() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let member_id = test_user_id(2);
  let request = create_room_request();

  let (room_id, _) = state.create_room(&request, owner_id.clone()).unwrap();

  let join_request = message::signaling::JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "member".to_string())
    .unwrap();

  // Ban
  let ban_request = BanMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  state.ban_member(&ban_request, &owner_id.clone()).unwrap();

  // Unban
  let unban_request = UnbanMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = state.unban_member(&unban_request, &owner_id);
  assert!(result.is_ok());

  // Should be able to rejoin now
  let rejoin_request = message::signaling::JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  let result = state.join_room(&rejoin_request, member_id, "member".to_string());
  assert!(result.is_ok());
}

#[test]
fn test_unban_not_banned_user() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let member_id = test_user_id(2);
  let request = create_room_request();

  let (room_id, _) = state.create_room(&request, owner_id.clone()).unwrap();

  let join_request = message::signaling::JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "member".to_string())
    .unwrap();

  let unban_request = UnbanMember {
    room_id: room_id.clone(),
    target: member_id,
  };
  let result = state.unban_member(&unban_request, &owner_id);
  assert!(result.is_err());
}

// ===========================================================================
// Announcement
// ===========================================================================

#[test]
fn test_set_announcement() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let request = create_room_request();

  let (room_id, _) = state.create_room(&request, owner_id.clone()).unwrap();

  let announcement = RoomAnnouncement {
    room_id: room_id.clone(),
    content: "Welcome to the room!".to_string(),
  };
  let result = state.set_announcement(&announcement, &owner_id);
  assert!(result.is_ok());

  let room = state.get_room(&room_id).unwrap();
  assert_eq!(room.info.announcement, "Welcome to the room!");
}

#[test]
fn test_set_announcement_non_owner() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let member_id = test_user_id(2);
  let request = create_room_request();

  let (room_id, _) = state.create_room(&request, owner_id.clone()).unwrap();

  let join_request = message::signaling::JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "member".to_string())
    .unwrap();

  let announcement = RoomAnnouncement {
    room_id: room_id.clone(),
    content: "Hacked!".to_string(),
  };
  let result = state.set_announcement(&announcement, &member_id);
  assert!(result.is_err());
}

// ===========================================================================
// Helpers
// ===========================================================================

/// Create a room with owner and an admin.
fn setup_room_with_admin(
  state: &RoomState,
  owner_id: &message::types::UserId,
  admin_id: &message::types::UserId,
) {
  let request = create_room_request();
  let (room_id, _) = state.create_room(&request, owner_id.clone()).unwrap();

  let join_request = message::signaling::JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, admin_id.clone(), "admin".to_string())
    .unwrap();

  let promote_request = message::signaling::PromoteAdmin {
    room_id,
    target: admin_id.clone(),
  };
  state.promote_admin(&promote_request, owner_id).unwrap();
}

/// Create a room with owner, admin, and a regular member.
fn setup_room_with_admin_and_member(
  state: &RoomState,
  owner_id: &message::types::UserId,
  admin_id: &message::types::UserId,
  member_id: &message::types::UserId,
) {
  setup_room_with_admin(state, owner_id, admin_id);

  let room_id = state.get_user_room(owner_id).unwrap();
  let join_request = message::signaling::JoinRoom {
    room_id,
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "member".to_string())
    .unwrap();
}

/// Create a room with owner and two regular members.
fn setup_room_with_two_members(
  state: &RoomState,
  owner_id: &message::types::UserId,
  member1_id: &message::types::UserId,
  member2_id: &message::types::UserId,
) {
  let request = create_room_request();
  let (room_id, _) = state.create_room(&request, owner_id.clone()).unwrap();

  let join1 = message::signaling::JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join1, member1_id.clone(), "m1".to_string())
    .unwrap();

  let join2 = message::signaling::JoinRoom {
    room_id,
    password: None,
  };
  state
    .join_room(&join2, member2_id.clone(), "m2".to_string())
    .unwrap();
}
