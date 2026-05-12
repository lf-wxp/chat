//! Utility method tests.

use super::*;

#[test]
fn test_is_empty_only_owner() {
  let room = create_test_room();
  assert!(!room.is_empty()); // Owner is still a member
}

#[test]
fn test_is_empty_after_owner_removed() {
  let mut room = create_test_room();
  let owner_id = room.info.owner_id.clone();
  room.remove_member(&owner_id);
  assert!(room.is_empty());
}

#[test]
fn test_to_room_info() {
  let room = create_test_room();
  let info = room.to_room_info();
  assert_eq!(info.name, room.info.name);
  assert_eq!(info.room_id, room.info.room_id);
  assert_eq!(info.owner_id, room.info.owner_id);
}

#[test]
fn test_get_members() {
  let mut room = create_test_room();
  let owner_id = room.info.owner_id.clone();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();

  let members = room.get_members();
  assert_eq!(members.len(), 2);
  assert!(members.iter().any(|m| m.user_id == owner_id));
  assert!(members.iter().any(|m| m.user_id == member_id));
}

#[test]
fn test_room_id() {
  let room_id = RoomId::new();
  let owner_id = UserId::new();
  let room = Room::new(
    room_id.clone(),
    "Test".to_string(),
    RoomType::Chat,
    owner_id,
  );

  assert_eq!(room.room_id(), &room_id);
}

#[test]
fn test_owner_id() {
  let owner_id = UserId::new();
  let room = Room::new(
    RoomId::new(),
    "Test".to_string(),
    RoomType::Chat,
    owner_id.clone(),
  );

  assert_eq!(room.owner_id(), &owner_id);
}

#[test]
fn test_member_count() {
  let mut room = create_test_room();
  assert_eq!(room.member_count(), 1);

  let m1 = UserId::new();
  room.add_member(m1, "A".to_string()).unwrap();
  assert_eq!(room.member_count(), 2);
}
