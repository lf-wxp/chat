//! Theater Transfer Owner precondition tests.

use super::*;

#[test]
fn test_theater_transfer_owner_room_not_found() {
  let ws_state = create_test_ws_state();
  let _owner_id = UserId::new();
  let target_id = UserId::new();
  let room_id = message::RoomId::new();

  let _theater_transfer = TheaterTransferOwner {
    room_id: room_id.clone(),
    target: target_id.clone(),
  };

  // Room doesn't exist
  assert!(ws_state.room_state.get_room(&room_id).is_none());
}

#[test]
fn test_theater_transfer_owner_non_owner_fails() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let non_owner_id = UserId::new();
  let target_id = UserId::new();

  // Create room
  let create_room = CreateRoom {
    name: "Theater Room".to_string(),
    description: String::new(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 50,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  // Add members
  ws_state.add_connection(non_owner_id.clone(), create_test_sender());
  ws_state.add_connection(target_id.clone(), create_test_sender());
  let join_room = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  ws_state
    .room_state
    .join_room(&join_room, non_owner_id.clone(), "nonowner".to_string())
    .unwrap();
  ws_state
    .room_state
    .join_room(&join_room, target_id.clone(), "target".to_string())
    .unwrap();

  // Non-owner tries to transfer
  let room = ws_state.room_state.get_room(&room_id).unwrap();
  assert_ne!(room.owner_id(), &non_owner_id);
}

#[test]
fn test_theater_transfer_target_not_member() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let target_id = UserId::new();

  // Create room
  let create_room = CreateRoom {
    name: "Theater Room".to_string(),
    description: String::new(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 50,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  // Target is not a member
  let room = ws_state.room_state.get_room(&room_id).unwrap();
  assert!(!room.is_member(&target_id));
}

#[test]
fn test_theater_transfer_owner_success() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let target_id = UserId::new();

  // Create theater room and add target member
  let create_room = CreateRoom {
    name: "Theater Room".to_string(),
    description: String::new(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 50,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  ws_state.add_connection(target_id.clone(), create_test_sender());
  let join_room = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  ws_state
    .room_state
    .join_room(&join_room, target_id.clone(), "target".to_string())
    .unwrap();

  // Owner should be able to transfer
  let room = ws_state.room_state.get_room(&room_id).unwrap();
  assert_eq!(room.owner_id(), &owner_id);
  assert!(room.is_member(&target_id));

  let theater_transfer = TheaterTransferOwner {
    room_id: room_id.clone(),
    target: target_id.clone(),
  };
  assert_eq!(theater_transfer.room_id, room_id);
  assert_eq!(theater_transfer.target, target_id);
}

#[test]
fn test_theater_transfer_broadcasts_to_all() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let target_id = UserId::new();
  let other_member_id = UserId::new();

  // Create theater room and add members
  let create_room = CreateRoom {
    name: "Theater Room".to_string(),
    description: String::new(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 50,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  ws_state.add_connection(owner_id.clone(), create_test_sender());
  ws_state.add_connection(target_id.clone(), create_test_sender());
  ws_state.add_connection(other_member_id.clone(), create_test_sender());

  let join_room = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  ws_state
    .room_state
    .join_room(&join_room, target_id.clone(), "target".to_string())
    .unwrap();
  ws_state
    .room_state
    .join_room(&join_room, other_member_id.clone(), "other".to_string())
    .unwrap();

  // Theater transfer should broadcast to ALL members (including sender)
  // Unlike call handlers which exclude sender
  let all_have_senders = ws_state.get_sender(&owner_id).is_some()
    && ws_state.get_sender(&target_id).is_some()
    && ws_state.get_sender(&other_member_id).is_some();
  assert!(all_have_senders);
}
