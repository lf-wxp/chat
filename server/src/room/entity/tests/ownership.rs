//! Ownership transfer and successor logic tests.

use super::*;

#[test]
fn test_transfer_ownership_success() {
  let mut room = create_test_room();
  let old_owner = room.info.owner_id.clone();
  let new_owner = UserId::new();
  room
    .add_member(new_owner.clone(), "Alice".to_string())
    .unwrap();

  let result = room.transfer_ownership(&new_owner);
  assert!(result.is_ok());
  assert_eq!(room.info.owner_id, new_owner);
  assert_eq!(room.get_member(&new_owner).unwrap().role, RoomRole::Owner);
  assert_eq!(room.get_member(&old_owner).unwrap().role, RoomRole::Admin);
}

#[test]
fn test_transfer_ownership_to_non_member() {
  let mut room = create_test_room();
  let non_member = UserId::new();

  let result = room.transfer_ownership(&non_member);
  assert_eq!(result.unwrap_err(), RoomError::NotMember);
}

#[test]
fn test_get_successor_oldest_admin_first() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  let admin_id = UserId::new();

  // Member joins first, then admin
  room
    .add_member(member_id.clone(), "Member".to_string())
    .unwrap();
  room
    .add_member(admin_id.clone(), "Admin".to_string())
    .unwrap();
  room.promote_admin(&admin_id).unwrap();

  // Admin should be successor even though member joined first
  let successor = room.get_successor();
  assert_eq!(successor, Some(admin_id));
}

#[test]
fn test_get_successor_oldest_member_when_no_admin() {
  let mut room = create_test_room();
  let member1 = UserId::new();
  let member2 = UserId::new();

  room.add_member(member1.clone(), "M1".to_string()).unwrap();
  room.add_member(member2, "M2".to_string()).unwrap();

  // Oldest member should be successor
  let successor = room.get_successor();
  assert_eq!(successor, Some(member1));
}

#[test]
fn test_get_successor_none_when_only_owner() {
  let room = create_test_room();
  assert!(room.get_successor().is_none());
}
