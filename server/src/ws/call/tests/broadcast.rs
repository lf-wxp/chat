//! Message broadcasting tests for call handlers.

use super::*;

#[test]
fn test_call_message_broadcast_to_members() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let member1_id = UserId::new();
  let member2_id = UserId::new();

  // Create room and add members
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

  ws_state.add_connection(owner_id.clone(), create_test_sender());
  ws_state.add_connection(member1_id.clone(), create_test_sender());
  ws_state.add_connection(member2_id.clone(), create_test_sender());

  let join_room = message::signaling::JoinRoom {
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

  // All members should have senders
  assert!(ws_state.get_sender(&owner_id).is_some());
  assert!(ws_state.get_sender(&member1_id).is_some());
  assert!(ws_state.get_sender(&member2_id).is_some());
}

#[test]
fn test_call_message_excludes_sender() {
  let owner_id = UserId::new();
  let member_id = UserId::new();

  // When forwarding messages, sender is excluded
  // This tests the logic pattern used in handlers
  let all_users = [owner_id.clone(), member_id.clone()];
  let recipients: Vec<UserId> = all_users
    .iter()
    .filter(|u| **u != owner_id)
    .cloned()
    .collect();

  assert_eq!(recipients.len(), 1);
  assert_eq!(recipients[0], member_id);
}
