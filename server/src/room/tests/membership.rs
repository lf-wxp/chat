//! Member management tests.

use super::*;

#[test]
fn test_promote_demote_admin() {
  let state = create_test_room_state();
  let owner_id = message::types::UserId::new();
  let member_id = message::types::UserId::new();

  // Create and join room
  let create_request = CreateRoom {
    name: "Test Room".to_string(),
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

  // Promote to admin
  let promote_request = PromoteAdmin {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = state.promote_admin(&promote_request, &owner_id);
  assert!(result.is_ok());

  // Verify admin role
  let room = state.get_room(&room_id).unwrap();
  let member = room.get_member(&member_id).unwrap();
  assert_eq!(member.role, message::types::RoomRole::Admin);

  // Demote back to member
  let demote_request = DemoteAdmin {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  let result = state.demote_admin(&demote_request, &owner_id);
  assert!(result.is_ok());

  // Verify member role
  let room = state.get_room(&room_id).unwrap();
  let member = room.get_member(&member_id).unwrap();
  assert_eq!(member.role, message::types::RoomRole::Member);
}

#[test]
fn test_set_announcement() {
  let state = create_test_room_state();
  let owner_id = message::types::UserId::new();
  let member_id = message::types::UserId::new();

  // Create and join room
  let create_request = CreateRoom {
    name: "Test Room".to_string(),
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

  // Owner sets announcement
  let announcement = RoomAnnouncement {
    room_id: room_id.clone(),
    content: "Welcome to the room!".to_string(),
  };
  let result = state.set_announcement(&announcement, &owner_id);
  assert!(result.is_ok());

  // Member cannot set announcement
  let announcement = RoomAnnouncement {
    room_id: room_id.clone(),
    content: "Hacked!".to_string(),
  };
  let result = state.set_announcement(&announcement, &member_id);
  assert_eq!(result.unwrap_err(), RoomError::InsufficientPermission);

  // Verify announcement
  let room = state.get_room(&room_id).unwrap();
  assert_eq!(room.info.announcement, "Welcome to the room!");
}

#[test]
fn test_set_nickname() {
  let state = create_test_room_state();
  let owner_id = message::types::UserId::new();
  let member_id = message::types::UserId::new();

  // Create and join room
  let create_request = CreateRoom {
    name: "Test Room".to_string(),
    room_type: message::types::RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = state.create_room(&create_request, owner_id).unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "OldName".to_string())
    .unwrap();

  // Change nickname
  let nickname_change = NicknameChange {
    user_id: member_id.clone(),
    new_nickname: "NewName".to_string(),
  };
  let result = state.set_nickname(&nickname_change);
  assert!(result.is_ok());

  // Verify nickname
  let room = state.get_room(&room_id).unwrap();
  let member = room.get_member(&member_id).unwrap();
  assert_eq!(member.nickname, "NewName");
}
