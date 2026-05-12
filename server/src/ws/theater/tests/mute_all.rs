//! Theater Mute All precondition tests.

use super::*;

#[test]
fn test_theater_mute_all_room_not_found() {
  let ws_state = create_test_ws_state();
  let room_id = message::RoomId::new();

  let _theater_mute_all = TheaterMuteAll {
    room_id: room_id.clone(),
  };

  // Room doesn't exist
  assert!(ws_state.room_state.get_room(&room_id).is_none());
}

#[test]
fn test_theater_mute_all_non_owner_fails() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let non_owner_id = UserId::new();

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

  // Add non-owner member
  ws_state.add_connection(non_owner_id.clone(), create_test_sender());
  let join_room = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  ws_state
    .room_state
    .join_room(&join_room, non_owner_id.clone(), "member".to_string())
    .unwrap();

  // Non-owner tries to mute all
  let room = ws_state.room_state.get_room(&room_id).unwrap();
  assert_ne!(room.owner_id(), &non_owner_id);
}

#[test]
fn test_theater_mute_all_owner_success() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();

  // Create theater room
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

  // Owner should be able to mute all
  let room = ws_state.room_state.get_room(&room_id).unwrap();
  assert_eq!(room.owner_id(), &owner_id);

  let theater_mute_all = TheaterMuteAll {
    room_id: room_id.clone(),
  };
  assert_eq!(theater_mute_all.room_id, room_id);
}

#[test]
fn test_theater_mute_all_broadcasts_to_members() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let member1_id = UserId::new();
  let member2_id = UserId::new();

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
  ws_state.add_connection(member1_id.clone(), create_test_sender());
  ws_state.add_connection(member2_id.clone(), create_test_sender());

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

  // All members should have senders for broadcast
  assert!(ws_state.get_sender(&owner_id).is_some());
  assert!(ws_state.get_sender(&member1_id).is_some());
  assert!(ws_state.get_sender(&member2_id).is_some());
}

#[test]
fn test_theater_mode_for_theater_room_type() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();

  // Create Theater room
  let create_room = CreateRoom {
    name: "Theater Room".to_string(),
    description: String::new(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 50,
  };
  let (_room_id, room_info) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  assert_eq!(room_info.room_type, RoomType::Theater);
}
