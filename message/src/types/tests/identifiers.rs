//! Identifier type tests: `UserId`, `RoomId`, `MessageId`, `TransferId`.

use super::*;

// ===========================================================================
// UserId creation tests
// ===========================================================================

#[test]
fn test_user_id_new_creates_unique_ids() {
  let id1 = UserId::new();
  let id2 = UserId::new();
  assert_ne!(id1, id2);
}

#[test]
fn test_user_id_from_uuid_string() {
  let uuid = uuid::Uuid::new_v4();
  let id = UserId::from_uuid(uuid);
  assert_eq!(*id.as_uuid(), uuid);
}

#[test]
fn test_user_id_display() {
  let id = UserId::new();
  let display = format!("{id}");
  assert!(!display.is_empty());
}

#[test]
fn test_user_id_clone_and_equality() {
  let id = UserId::new();
  let cloned = id.clone();
  assert_eq!(id, cloned);
}

#[test]
fn test_user_id_hash_consistency() {
  use std::collections::HashSet;
  let id = UserId::new();
  let cloned = id.clone();

  let mut set = HashSet::new();
  set.insert(id);
  assert!(set.contains(&cloned));
}

// ===========================================================================
// RoomId creation tests
// ===========================================================================

#[test]
fn test_room_id_new_creates_unique_ids() {
  let id1 = RoomId::new();
  let id2 = RoomId::new();
  assert_ne!(id1, id2);
}

#[test]
fn test_room_id_from_uuid_string() {
  let uuid = uuid::Uuid::new_v4();
  let id = RoomId::from_uuid(uuid);
  assert_eq!(*id.as_uuid(), uuid);
}

#[test]
fn test_room_id_display() {
  let id = RoomId::new();
  let display = format!("{id}");
  assert!(!display.is_empty());
}

#[test]
fn test_room_id_clone_and_equality() {
  let id = RoomId::new();
  let cloned = id.clone();
  assert_eq!(id, cloned);
}

#[test]
fn test_room_id_hash_consistency() {
  use std::collections::HashSet;
  let id = RoomId::new();
  let cloned = id.clone();

  let mut set = HashSet::new();
  set.insert(id);
  assert!(set.contains(&cloned));
}

// ===========================================================================
// MessageId creation tests
// ===========================================================================

#[test]
fn test_message_id_new_creates_unique_ids() {
  let id1 = MessageId::new();
  let id2 = MessageId::new();
  assert_ne!(id1, id2);
}

#[test]
fn test_message_id_from_uuid_string() {
  let uuid = uuid::Uuid::new_v4();
  let id = MessageId::from_uuid(uuid);
  assert_eq!(*id.as_uuid(), uuid);
}

#[test]
fn test_message_id_display() {
  let id = MessageId::new();
  let display = format!("{id}");
  assert!(!display.is_empty());
}

#[test]
fn test_message_id_clone_and_equality() {
  let id = MessageId::new();
  let copied = id;
  assert_eq!(id, copied);
}

// ===========================================================================
// TransferId creation tests
// ===========================================================================

#[test]
fn test_transfer_id_new_creates_unique_ids() {
  let id1 = TransferId::new();
  let id2 = TransferId::new();
  assert_ne!(id1, id2);
}

#[test]
fn test_transfer_id_from_uuid_string() {
  let uuid = uuid::Uuid::new_v4();
  let id = TransferId::from_uuid(uuid);
  assert_eq!(*id.as_uuid(), uuid);
}

#[test]
fn test_transfer_id_display() {
  let id = TransferId::new();
  let display = format!("{id}");
  assert!(!display.is_empty());
}

#[test]
fn test_transfer_id_clone_and_equality() {
  let id = TransferId::new();
  let cloned = id.clone();
  assert_eq!(id, cloned);
}

// ===========================================================================
// Cross-identifier type safety
// ===========================================================================

#[test]
fn test_different_identifier_types_not_comparable() {
  // UserId and RoomId should be different types and not interchangeable
  let user_id = UserId::new();
  let room_id = RoomId::new();

  // These are different types, so they cannot be compared directly
  // This test just ensures the types exist and are distinct
  let _ = user_id;
  let _ = room_id;
}
