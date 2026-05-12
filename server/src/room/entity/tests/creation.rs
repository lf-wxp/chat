//! Room creation tests.

use super::*;

#[test]
fn test_new_room_has_owner_as_member() {
  let owner_id = UserId::new();
  let room_id = RoomId::new();
  let room = Room::new(
    room_id.clone(),
    "Test".to_string(),
    RoomType::Chat,
    owner_id.clone(),
  );

  assert_eq!(room.info.room_id, room_id);
  assert_eq!(room.info.name, "Test");
  assert_eq!(room.info.room_type, RoomType::Chat);
  assert_eq!(room.info.owner_id, owner_id);
  assert_eq!(room.member_count(), 1);
  assert!(room.is_member(&owner_id));
  assert_eq!(room.get_member(&owner_id).unwrap().role, RoomRole::Owner);
  assert_eq!(room.join_order.len(), 1);
  assert!(room.banned_users.is_empty());
}

#[test]
fn test_new_room_default_max_members() {
  let room = create_test_room();
  assert_eq!(room.info.max_members, DEFAULT_MAX_MEMBERS);
}
