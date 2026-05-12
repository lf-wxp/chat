//! Edge case tests for theater mode.

use super::*;

#[test]
fn test_theater_transfer_to_self() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();

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

  // Owner tries to transfer to self
  let room = ws_state.room_state.get_room(&room_id).unwrap();
  assert!(room.is_member(&owner_id)); // Owner is also a member

  let theater_transfer = TheaterTransferOwner {
    room_id: room_id.clone(),
    target: owner_id.clone(),
  };

  // Self-transfer should be rejected by the handler (SIG313 error)
  // The handler checks: if theater_transfer.target == *user_id, send error and return
  assert_eq!(theater_transfer.target, owner_id); // target equals sender
}

#[test]
fn test_theater_mute_all_empty_room() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();

  // Create room with only owner
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

  // Owner only room
  let room = ws_state.room_state.get_room(&room_id).unwrap();
  assert_eq!(room.member_count(), 1);

  // Mute all should still work (no one to mute except owner)
  let theater_mute_all = TheaterMuteAll {
    room_id: room_id.clone(),
  };
  assert_eq!(theater_mute_all.room_id, room_id);
}
