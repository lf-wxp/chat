//! Member management tests.

use super::*;

#[test]
fn test_is_full_false_when_below_capacity() {
  let room = create_test_room();
  assert!(!room.is_full());
}

#[test]
fn test_is_full_true_at_capacity() {
  let mut room = create_test_room();
  room.info.max_members = 2;

  let member_id = UserId::new();
  room.add_member(member_id, "Member1".to_string()).unwrap();
  assert!(room.is_full());
}

#[test]
fn test_add_member_success() {
  let mut room = create_test_room();
  let member_id = UserId::new();

  let result = room.add_member(member_id.clone(), "Alice".to_string());
  assert!(result.is_ok());
  assert_eq!(room.member_count(), 2);
  assert!(room.is_member(&member_id));

  let member = room.get_member(&member_id).unwrap();
  assert_eq!(member.nickname, "Alice");
  assert_eq!(member.role, RoomRole::Member);
  assert!(room.join_order.contains(&member_id));
}

#[test]
fn test_add_member_already_member() {
  let mut room = create_test_room();
  let owner_id = room.info.owner_id.clone();

  let result = room.add_member(owner_id, "Again".to_string());
  assert_eq!(result.unwrap_err(), RoomError::AlreadyMember);
}

#[test]
fn test_add_member_room_full() {
  let mut room = create_test_room();
  room.info.max_members = 1;

  let member_id = UserId::new();
  let result = room.add_member(member_id, "Member1".to_string());
  assert_eq!(result.unwrap_err(), RoomError::RoomFull);
}

#[test]
fn test_add_member_banned_user() {
  let mut room = create_test_room();
  let member_id = UserId::new();

  room.ban_user(member_id.clone()).unwrap();
  let result = room.add_member(member_id, "Banned".to_string());
  assert_eq!(result.unwrap_err(), RoomError::UserBanned);
}

#[test]
fn test_remove_member_success() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();

  let removed = room.remove_member(&member_id);
  assert!(removed.is_some());
  assert_eq!(removed.unwrap().nickname, "Alice");
  assert!(!room.is_member(&member_id));
  assert!(!room.join_order.contains(&member_id));
  assert_eq!(room.member_count(), 1);
}

#[test]
fn test_remove_member_not_found() {
  let mut room = create_test_room();
  let unknown_id = UserId::new();

  let removed = room.remove_member(&unknown_id);
  assert!(removed.is_none());
}

#[test]
fn test_is_member() {
  let room = create_test_room();
  let owner_id = room.info.owner_id.clone();
  let non_member = UserId::new();

  assert!(room.is_member(&owner_id));
  assert!(!room.is_member(&non_member));
}

#[test]
fn test_get_member_mut() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();

  let member = room.get_member_mut(&member_id).unwrap();
  member.nickname = "Bob".to_string();

  assert_eq!(room.get_member(&member_id).unwrap().nickname, "Bob");
}

#[test]
fn test_touch_member() {
  let mut room = create_test_room();
  let owner_id = room.info.owner_id.clone();

  room.touch_member(&owner_id);
  // Should not panic and should update last_active
  let member = room.get_member(&owner_id).unwrap();
  assert!(member.last_active() <= chrono::Utc::now());
}

#[test]
fn test_touch_member_nonexistent() {
  let mut room = create_test_room();
  let unknown_id = UserId::new();

  // Should not panic
  room.touch_member(&unknown_id);
}
