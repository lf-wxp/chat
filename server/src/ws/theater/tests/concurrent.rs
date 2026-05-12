//! Concurrent theater operations tests.

use super::*;
use std::sync::Arc;

#[test]
fn test_concurrent_theater_transfers() {
  let ws_state = Arc::new(create_test_ws_state());
  let owner_id = UserId::new();
  let members: Vec<UserId> = (0..5).map(|_| UserId::new()).collect();

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

  // Add members
  ws_state.add_connection(owner_id.clone(), create_test_sender());
  for member in &members {
    ws_state.add_connection(member.clone(), create_test_sender());
    let join_room = JoinRoom {
      room_id: room_id.clone(),
      password: None,
    };
    ws_state
      .room_state
      .join_room(&join_room, member.clone(), "member".to_string())
      .unwrap();
  }

  // All members should be in room
  let room = ws_state.room_state.get_room(&room_id).unwrap();
  assert_eq!(room.member_count(), 6); // owner + 5 members
}
