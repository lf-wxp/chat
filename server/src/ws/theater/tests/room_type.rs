//! Room type validation tests for theater mode.

use super::*;

#[test]
fn test_theater_mode_for_chat_room() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();

  // Create Chat room (theater mode might still apply)
  let create_room = CreateRoom {
    name: "Chat Room".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (_room_id, room_info) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  assert_eq!(room_info.room_type, RoomType::Chat);
}
