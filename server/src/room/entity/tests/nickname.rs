//! Nickname management tests.

use super::*;

#[test]
fn test_set_nickname_success() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();

  let old = room.set_nickname(&member_id, "Bob".to_string()).unwrap();
  assert_eq!(old, "Alice");
  assert_eq!(room.get_member(&member_id).unwrap().nickname, "Bob");
}

#[test]
fn test_set_nickname_too_long() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();

  let long_nickname = "a".repeat(MAX_NICKNAME_LENGTH + 1);
  let result = room.set_nickname(&member_id, long_nickname);
  assert!(result.is_err());
  match result.unwrap_err() {
    RoomError::InvalidInput(msg) => assert!(msg.contains("maximum length")),
    _ => panic!("Expected InvalidInput error"),
  }
}

#[test]
fn test_set_nickname_not_member() {
  let mut room = create_test_room();
  let unknown_id = UserId::new();

  let result = room.set_nickname(&unknown_id, "Bob".to_string());
  assert_eq!(result.unwrap_err(), RoomError::NotMember);
}
