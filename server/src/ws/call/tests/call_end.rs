//! Call End precondition tests.

use super::*;

#[test]
fn test_call_end_room_not_found() {
  let ws_state = create_test_ws_state();
  let _user_id = UserId::new();
  let room_id = message::RoomId::new();

  let _call_end = CallEnd {
    from: UserId::new(),
    room_id: room_id.clone(),
  };

  // Room doesn't exist
  assert!(ws_state.room_state.get_room(&room_id).is_none());
}

#[test]
fn test_call_end_member_success() {
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

  // Call end should be valid
  let call_end = CallEnd {
    from: member_id.clone(),
    room_id: room_id.clone(),
  };
  assert_eq!(call_end.room_id, room_id);
}
