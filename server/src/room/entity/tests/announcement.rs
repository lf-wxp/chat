//! Announcement management tests.

use super::*;

#[test]
fn test_set_announcement_success() {
  let mut room = create_test_room();
  room.set_announcement("Hello world!".to_string()).unwrap();
  assert_eq!(room.info.announcement, "Hello world!");
}

#[test]
fn test_set_announcement_too_long() {
  let mut room = create_test_room();
  let long_announcement = "a".repeat(MAX_ANNOUNCEMENT_LENGTH + 1);

  let result = room.set_announcement(long_announcement);
  assert!(result.is_err());
  match result.unwrap_err() {
    RoomError::InvalidInput(msg) => assert!(msg.contains("maximum length")),
    _ => panic!("Expected InvalidInput error"),
  }
}

#[test]
fn test_set_announcement_at_max_length() {
  let mut room = create_test_room();
  let announcement = "a".repeat(MAX_ANNOUNCEMENT_LENGTH);

  let result = room.set_announcement(announcement);
  assert!(result.is_ok());
}
