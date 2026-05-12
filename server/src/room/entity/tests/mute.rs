//! Mute/Unmute tests.

use super::*;

#[test]
fn test_mute_member_timed() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();

  let result = room.mute_member(&member_id, Some(3600));
  assert!(result.is_ok());

  let mute_info = result.unwrap();
  assert!(matches!(mute_info, MuteInfo::Timed { .. }));
  assert!(room.get_member(&member_id).unwrap().is_muted());
}

#[test]
fn test_mute_member_permanent() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();

  let result = room.mute_member(&member_id, None);
  assert!(result.is_ok());

  let mute_info = result.unwrap();
  assert_eq!(mute_info, MuteInfo::Permanent);
  assert!(room.get_member(&member_id).unwrap().is_muted());
}

#[test]
fn test_mute_member_not_in_room() {
  let mut room = create_test_room();
  let unknown_id = UserId::new();

  let result = room.mute_member(&unknown_id, Some(60));
  assert_eq!(result.unwrap_err(), RoomError::NotMember);
}

#[test]
fn test_unmute_member_success() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();
  room.mute_member(&member_id, None).unwrap();

  let result = room.unmute_member(&member_id);
  assert!(result.is_ok());
  assert_eq!(
    room.get_member(&member_id).unwrap().mute_info,
    MuteInfo::NotMuted
  );
}

#[test]
fn test_unmute_member_not_in_room() {
  let mut room = create_test_room();
  let unknown_id = UserId::new();

  let result = room.unmute_member(&unknown_id);
  assert_eq!(result.unwrap_err(), RoomError::NotMember);
}

#[test]
fn test_check_expired_mutes_returns_expired() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();

  // Mute for 1 second
  room.mute_member(&member_id, Some(1)).unwrap();

  // Wait for expiry
  std::thread::sleep(std::time::Duration::from_millis(1500));

  let expired = room.check_expired_mutes();
  assert!(expired.contains(&member_id));
  assert_eq!(
    room.get_member(&member_id).unwrap().mute_info,
    MuteInfo::NotMuted
  );
}

#[test]
fn test_check_expired_mutes_permanent_not_expired() {
  let mut room = create_test_room();
  let member_id = UserId::new();
  room
    .add_member(member_id.clone(), "Alice".to_string())
    .unwrap();

  room.mute_member(&member_id, None).unwrap();

  let expired = room.check_expired_mutes();
  assert!(!expired.contains(&member_id));
  assert!(room.get_member(&member_id).unwrap().is_muted());
}
