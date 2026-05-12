//! Ban/Unban tests.

use super::*;

#[test]
fn test_is_banned() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();

  assert!(!room.is_banned(&member_id));
  room.ban_user(member_id.clone()).unwrap();
  assert!(room.is_banned(&member_id));
}

#[test]
fn test_ban_user_removes_from_members() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();

  room.ban_user(member_id.clone()).unwrap();
  assert!(!room.is_member(&member_id));
  assert!(room.is_banned(&member_id));
}

#[test]
fn test_ban_user_already_banned_not_duplicated() {
  let mut room = create_test_room();
  let member_id = UserId::new();

  room.ban_user(member_id.clone()).unwrap();
  room.ban_user(member_id.clone()).unwrap();
  assert_eq!(room.banned_users.len(), 1);
}

#[test]
fn test_unban_user_success() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room.ban_user(member_id.clone()).unwrap();

  let result = room.unban_user(&member_id);
  assert!(result);
  assert!(!room.is_banned(&member_id));
}

#[test]
fn test_unban_user_not_banned() {
  let mut room = create_test_room();
  let member_id = UserId::new();

  let result = room.unban_user(&member_id);
  assert!(!result);
}
