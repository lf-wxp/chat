//! Role management tests: promote, demote, transfer ownership, nickname.

use message::signaling::{DemoteAdmin, JoinRoom, PromoteAdmin, TransferOwnership};
use message::types::RoomRole;

use super::super::RoomState;
use super::*;

// ===========================================================================
// Promote / Demote Admin
// ===========================================================================

#[test]
fn test_promote_member_to_admin() {
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

  let promote_request = PromoteAdmin {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = state.promote_admin(&promote_request, &owner_id);
  assert!(result.is_ok());

  let member_info = result.unwrap();
  assert_eq!(member_info.role, RoomRole::Admin);
}

#[test]
fn test_promote_non_owner_fails() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let member1_id = test_user_id(2);
  let member2_id = test_user_id(3);
  let request = create_room_request();

  let (room_id, _) = state.create_room(&request, owner_id).unwrap();

  let join1 = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join1, member1_id.clone(), "m1".to_string())
    .unwrap();

  let join2 = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join2, member2_id.clone(), "m2".to_string())
    .unwrap();

  // Non-owner cannot promote
  let promote_request = PromoteAdmin {
    room_id: room_id.clone(),
    target: member2_id,
  };
  let result = state.promote_admin(&promote_request, &member1_id);
  assert!(result.is_err());
}

#[test]
fn test_demote_admin_to_member() {
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

  // Promote first
  let promote_request = PromoteAdmin {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  state.promote_admin(&promote_request, &owner_id).unwrap();

  // Demote back
  let demote_request = DemoteAdmin {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = state.demote_admin(&demote_request, &owner_id);
  assert!(result.is_ok());

  let member_info = result.unwrap();
  assert_eq!(member_info.role, RoomRole::Member);
}

#[test]
fn test_cannot_promote_owner() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let request = create_room_request();

  let (room_id, _) = state.create_room(&request, owner_id.clone()).unwrap();

  let promote_request = PromoteAdmin {
    room_id,
    target: owner_id.clone(),
  };
  let result = state.promote_admin(&promote_request, &owner_id);
  assert!(result.is_err());
}

#[test]
fn test_cannot_demote_owner() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let request = create_room_request();

  let (room_id, _) = state.create_room(&request, owner_id.clone()).unwrap();

  let demote_request = DemoteAdmin {
    room_id,
    target: owner_id.clone(),
  };
  let result = state.demote_admin(&demote_request, &owner_id);
  assert!(result.is_err());
}

#[test]
fn test_demote_non_admin_fails() {
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

  // Cannot demote a regular member (they are not an admin)
  let demote_request = DemoteAdmin {
    room_id,
    target: member_id,
  };
  let result = state.demote_admin(&demote_request, &owner_id);
  assert!(result.is_err());
}

// ===========================================================================
// Transfer Ownership
// ===========================================================================

#[test]
fn test_transfer_ownership() {
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

  let transfer_request = TransferOwnership {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = state.transfer_ownership(&transfer_request, &owner_id);
  assert!(result.is_ok());

  let (old_owner, new_owner) = result.unwrap();
  assert_eq!(old_owner.role, RoomRole::Admin); // Old owner becomes admin
  assert_eq!(new_owner.role, RoomRole::Owner); // New member becomes owner
}

#[test]
fn test_transfer_ownership_non_owner_fails() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let member1_id = test_user_id(2);
  let member2_id = test_user_id(3);
  let request = create_room_request();

  let (room_id, _) = state.create_room(&request, owner_id).unwrap();

  let join1 = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join1, member1_id.clone(), "m1".to_string())
    .unwrap();

  let join2 = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join2, member2_id.clone(), "m2".to_string())
    .unwrap();

  // Non-owner cannot transfer ownership
  let transfer_request = TransferOwnership {
    room_id: room_id.clone(),
    target: member2_id,
  };
  let result = state.transfer_ownership(&transfer_request, &member1_id);
  assert!(result.is_err());
}

#[test]
fn test_transfer_ownership_to_non_member_fails() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let non_member_id = test_user_id(2);
  let request = create_room_request();

  let (room_id, _) = state.create_room(&request, owner_id.clone()).unwrap();

  let transfer_request = TransferOwnership {
    room_id,
    target: non_member_id,
  };
  let result = state.transfer_ownership(&transfer_request, &owner_id);
  assert!(result.is_err());
}

// ===========================================================================
// Nickname
// ===========================================================================

#[test]
fn test_set_nickname() {
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

  let nickname_request = message::signaling::NicknameChange {
    user_id: member_id.clone(),
    new_nickname: "cool_name".to_string(),
  };
  let result = state.set_nickname(&nickname_request);
  assert!(result.is_ok());

  // Verify nickname changed
  let room = state.get_room(&room_id).unwrap();
  let member = room.get_member(&member_id).unwrap();
  assert_eq!(member.nickname, "cool_name");
}

#[test]
fn test_set_nickname_not_in_room() {
  let state = RoomState::new();
  let user_id = test_user_id(1);

  let nickname_request = message::signaling::NicknameChange {
    user_id,
    new_nickname: "cool_name".to_string(),
  };
  let result = state.set_nickname(&nickname_request);
  assert!(result.is_err());
}
