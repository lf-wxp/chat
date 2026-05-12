//! Call Accept precondition tests.

use super::*;

#[test]
fn test_call_accept_room_not_found() {
  let ws_state = create_test_ws_state();
  let _user_id = UserId::new();
  let room_id = message::RoomId::new();

  let _call_accept = CallAccept {
    from: UserId::new(),
    room_id: room_id.clone(),
  };

  // Room doesn't exist
  assert!(ws_state.room_state.get_room(&room_id).is_none());
}

#[test]
fn test_call_accept_user_not_member() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let non_member_id = UserId::new();

  // Create room
  let create_room = CreateRoom {
    name: "Test Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  let _call_accept = CallAccept {
    from: non_member_id.clone(),
    room_id: room_id.clone(),
  };

  // Non-member tries to accept
  let room = ws_state.room_state.get_room(&room_id).unwrap();
  assert!(!room.is_member(&non_member_id));
}

#[test]
fn test_call_accept_member_success() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  // Create room and add member
  let create_room = CreateRoom {
    name: "Test Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  let join_room = message::signaling::JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  ws_state
    .room_state
    .join_room(&join_room, member_id.clone(), "member".to_string())
    .unwrap();

  // Member is now in room
  let room = ws_state.room_state.get_room(&room_id).unwrap();
  assert!(room.is_member(&member_id));

  // Call accept should be valid
  let call_accept = CallAccept {
    from: member_id.clone(),
    room_id: room_id.clone(),
  };
  assert_eq!(call_accept.room_id, room_id);
}
