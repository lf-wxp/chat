//! Role management tests.

use super::*;

#[test]
fn test_promote_admin_success() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();

  let result = room.promote_admin(&member_id);
  assert!(result.is_ok());
  assert_eq!(room.get_member(&member_id).unwrap().role, RoomRole::Admin);
}

#[test]
fn test_promote_admin_cannot_promote_owner() {
  let mut room = create_test_room();
  let owner_id = room.info.owner_id.clone();

  let result = room.promote_admin(&owner_id);
  assert_eq!(result.unwrap_err(), RoomError::CannotPromoteOwner);
}

#[test]
fn test_promote_admin_not_member() {
  let mut room = create_test_room();
  let unknown_id = UserId::new();

  let result = room.promote_admin(&unknown_id);
  assert_eq!(result.unwrap_err(), RoomError::NotMember);
}

#[test]
fn test_demote_admin_success() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();
  room.promote_admin(&member_id).unwrap();

  let result = room.demote_admin(&member_id);
  assert!(result.is_ok());
  assert_eq!(room.get_member(&member_id).unwrap().role, RoomRole::Member);
}

#[test]
fn test_demote_admin_cannot_demote_owner() {
  let mut room = create_test_room();
  let owner_id = room.info.owner_id.clone();

  let result = room.demote_admin(&owner_id);
  assert_eq!(result.unwrap_err(), RoomError::CannotDemoteOwner);
}

#[test]
fn test_demote_admin_not_admin() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();

  let result = room.demote_admin(&member_id);
  assert_eq!(result.unwrap_err(), RoomError::NotAdmin);
}
