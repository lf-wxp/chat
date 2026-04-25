//! RoomState creation and default value tests.

use super::super::RoomState;

#[test]
fn test_new_room_state_is_empty() {
  let state = RoomState::new();
  assert_eq!(state.room_count(), 0);
  assert_eq!(state.total_member_count(), 0);
  assert!(state.get_all_rooms().is_empty());
}

#[test]
fn test_default_room_state() {
  let state = RoomState::default();
  assert_eq!(state.room_count(), 0);
  assert_eq!(state.total_member_count(), 0);
}
