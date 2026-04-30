//! Edge case and utility method tests for RoomState.

use message::signaling::{JoinRoom, LeaveRoom, MuteMember};
use message::types::{RoomId, RoomType};

use super::super::RoomState;
use super::*;

// ===========================================================================
// Remove user from all rooms (disconnect)
// ===========================================================================

#[test]
fn test_remove_user_from_all_rooms() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let member_id = test_user_id(2);
  let request = create_room_request();

  let (room_id, _) = state.create_room(&request, owner_id).unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "member".to_string())
    .unwrap();

  // Remove user from all rooms
  state.remove_user_from_all_rooms(&member_id);

  // Member should no longer be in any room
  assert!(state.get_user_room(&member_id).is_none());

  // Room should still exist with one fewer member
  let room = state.get_room(&room_id).unwrap();
  assert_eq!(room.info.member_count, 1);
}

// ===========================================================================
// Utility methods
// ===========================================================================

#[test]
fn test_get_room_found() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);

  let (room_id, _) = state
    .create_room(
      &create_room_request_with_params("Test", RoomType::Chat),
      owner_id,
    )
    .unwrap();

  assert!(state.get_room(&room_id).is_some());
}

#[test]
fn test_get_room_not_found() {
  let state = RoomState::new();
  let result = state.get_room(&RoomId::new());
  assert!(result.is_none());
}

#[test]
fn test_get_all_rooms_empty() {
  let state = RoomState::new();
  let rooms = state.get_all_rooms();
  assert!(rooms.is_empty());
}

#[test]
fn test_get_all_rooms_multiple() {
  let state = RoomState::new();
  let owner1 = test_user_id(1);
  let owner2 = test_user_id(2);

  let request1 = CreateRoom {
    name: "room1".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let request2 = CreateRoom {
    name: "room2".to_string(),
    description: String::new(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 8,
  };

  state.create_room(&request1, owner1).unwrap();
  state.create_room(&request2, owner2).unwrap();

  let rooms = state.get_all_rooms();
  assert_eq!(rooms.len(), 2);
}

#[test]
fn test_get_room_members_empty() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let request = create_room_request();

  let (room_id, _) = state.create_room(&request, owner_id).unwrap();
  let members = state.get_room_members(&room_id).unwrap();
  // Owner is the only member
  assert_eq!(members.len(), 1);
}

#[test]
fn test_room_count() {
  let state = RoomState::new();
  let owner1 = test_user_id(1);
  let owner2 = test_user_id(2);

  state
    .create_room(
      &create_room_request_with_params("Room1", RoomType::Chat),
      owner1,
    )
    .unwrap();
  assert_eq!(state.room_count(), 1);

  state
    .create_room(
      &create_room_request_with_params("Room2", RoomType::Theater),
      owner2,
    )
    .unwrap();
  assert_eq!(state.room_count(), 2);
}

#[test]
fn test_total_member_count() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let member_id = test_user_id(2);

  let (room_id, _) = state
    .create_room(
      &create_room_request_with_params("Test", RoomType::Chat),
      owner_id,
    )
    .unwrap();
  assert_eq!(state.total_member_count(), 1);

  let join_request = JoinRoom {
    room_id,
    password: None,
  };
  state
    .join_room(&join_request, member_id, "Alice".to_string())
    .unwrap();
  assert_eq!(state.total_member_count(), 2);
}

#[test]
fn test_total_member_count_multiple_rooms() {
  let state = RoomState::new();
  let owner1 = test_user_id(1);
  let owner2 = test_user_id(2);

  let request1 = CreateRoom {
    name: "room1".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let request2 = CreateRoom {
    name: "room2".to_string(),
    description: String::new(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };

  state.create_room(&request1, owner1).unwrap();
  state.create_room(&request2, owner2).unwrap();

  assert_eq!(state.room_count(), 2);
  assert_eq!(state.total_member_count(), 2);
}

// ===========================================================================
// get_user_room edge cases
// ===========================================================================

#[test]
fn test_get_user_room_no_room() {
  let state = RoomState::new();
  let user_id = test_user_id(1);
  let result = state.get_user_room(&user_id);
  assert!(result.is_none());
}

#[test]
fn test_get_user_room_after_leaving() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let member_id = test_user_id(2);

  let (room_id, _) = state.create_room(&create_room_request(), owner_id).unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Alice".to_string())
    .unwrap();

  // Member is in room
  assert_eq!(state.get_user_room(&member_id), Some(room_id.clone()));

  // Member leaves
  let leave_request = LeaveRoom {
    room_id: room_id.clone(),
  };
  state.leave_room(&leave_request, &member_id).unwrap();

  // After leaving, user should not be in any room
  assert_eq!(state.get_user_room(&member_id), None);
}

// ===========================================================================
// get_room_members edge cases
// ===========================================================================

#[test]
fn test_get_room_members_nonexistent_room() {
  let state = RoomState::new();
  let fake_room_id = RoomId::new();

  // Non-existent room should return None
  assert_eq!(state.get_room_members(&fake_room_id), None);
}

#[test]
fn test_get_room_members_multiple_members() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let member1_id = test_user_id(2);
  let member2_id = test_user_id(3);

  let (room_id, _) = state.create_room(&create_room_request(), owner_id).unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request.clone(), member1_id, "Alice".to_string())
    .unwrap();
  state
    .join_room(&join_request, member2_id, "Bob".to_string())
    .unwrap();

  let members = state.get_room_members(&room_id).unwrap();
  assert_eq!(members.len(), 3); // owner + 2 members
}

// ===========================================================================
// check_expired_mutes
// ===========================================================================

#[test]
fn test_check_expired_mutes_no_rooms() {
  let state = RoomState::new();

  // No rooms, should return empty map
  let expired = state.check_expired_mutes();
  assert!(expired.is_empty());
}

#[test]
fn test_check_expired_mutes_no_mutes() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let request = create_room_request();

  let _ = state.create_room(&request, owner_id).unwrap();

  // Should not panic with no mutes
  state.check_expired_mutes();
}

#[test]
fn test_check_expired_mutes_active_mutes() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let member_id = test_user_id(2);

  let (room_id, _) = state
    .create_room(
      &create_room_request_with_params("Test", RoomType::Chat),
      owner_id.clone(),
    )
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Alice".to_string())
    .unwrap();

  // Mute with long duration (should not expire)
  let mute_request = MuteMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
    duration_secs: Some(3600), // 1 hour
  };
  state.mute_member(&mute_request, &owner_id).unwrap();

  // Check immediately - no expired mutes
  let expired = state.check_expired_mutes();
  assert!(expired.is_empty());
}

#[test]
fn test_check_expired_mutes_expired_mutes() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let member_id = test_user_id(2);

  let (room_id, _) = state
    .create_room(
      &create_room_request_with_params("Test", RoomType::Chat),
      owner_id.clone(),
    )
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Alice".to_string())
    .unwrap();

  // Mute with very short duration (1 second)
  let mute_request = MuteMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
    duration_secs: Some(1),
  };
  state.mute_member(&mute_request, &owner_id).unwrap();

  // Wait for mute to expire
  std::thread::sleep(std::time::Duration::from_millis(1500));

  // Check - should find expired mute
  let expired = state.check_expired_mutes();
  assert_eq!(expired.len(), 1);
  assert!(expired.contains_key(&room_id));
  assert!(expired[&room_id].contains(&member_id));

  // Member should no longer be muted
  let room = state.get_room(&room_id).unwrap();
  assert!(!room.get_member(&member_id).unwrap().is_muted());
}

#[test]
fn test_check_expired_mutes_multiple_rooms() {
  let state = RoomState::new();
  let owner1_id = test_user_id(1);
  let owner2_id = test_user_id(2);
  let member1_id = test_user_id(3);
  let member2_id = test_user_id(4);

  // Create two rooms
  let (room1_id, _) = state
    .create_room(
      &create_room_request_with_params("Room1", RoomType::Chat),
      owner1_id.clone(),
    )
    .unwrap();
  let (room2_id, _) = state
    .create_room(
      &create_room_request_with_params("Room2", RoomType::Theater),
      owner2_id.clone(),
    )
    .unwrap();

  // Add members
  state
    .join_room(
      &JoinRoom {
        room_id: room1_id.clone(),
        password: None,
      },
      member1_id.clone(),
      "Alice".to_string(),
    )
    .unwrap();
  state
    .join_room(
      &JoinRoom {
        room_id: room2_id.clone(),
        password: None,
      },
      member2_id.clone(),
      "Bob".to_string(),
    )
    .unwrap();

  // Mute both with short duration
  state
    .mute_member(
      &MuteMember {
        room_id: room1_id.clone(),
        target: member1_id.clone(),
        duration_secs: Some(1),
      },
      &owner1_id,
    )
    .unwrap();
  state
    .mute_member(
      &MuteMember {
        room_id: room2_id.clone(),
        target: member2_id.clone(),
        duration_secs: Some(1),
      },
      &owner2_id,
    )
    .unwrap();

  // Wait for mutes to expire
  std::thread::sleep(std::time::Duration::from_millis(1500));

  // Check - should find expired mutes in both rooms
  let expired = state.check_expired_mutes();
  assert_eq!(expired.len(), 2);
  assert!(expired.contains_key(&room1_id));
  assert!(expired.contains_key(&room2_id));
}

#[test]
fn test_check_expired_mutes_permanent_mute_stays() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);
  let member_id = test_user_id(2);

  let (room_id, _) = state
    .create_room(
      &create_room_request_with_params("Test", RoomType::Chat),
      owner_id.clone(),
    )
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Alice".to_string())
    .unwrap();

  // Permanent mute (None duration)
  let mute_request = MuteMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
    duration_secs: None, // permanent
  };
  state.mute_member(&mute_request, &owner_id).unwrap();

  // Wait some time
  std::thread::sleep(std::time::Duration::from_millis(500));

  // Check - permanent mute should not expire
  let expired = state.check_expired_mutes();
  assert!(expired.is_empty());

  // Member should still be muted
  let room = state.get_room(&room_id).unwrap();
  assert!(room.get_member(&member_id).unwrap().is_muted());
}

// ===========================================================================
// Additional edge cases for room statistics
// ===========================================================================

#[test]
fn test_room_count_after_destruction() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);

  let (room_id, _) = state
    .create_room(&create_room_request(), owner_id.clone())
    .unwrap();
  assert_eq!(state.room_count(), 1);

  // Owner leaves - room should be destroyed (no other members)
  let leave_request = LeaveRoom { room_id };
  state.leave_room(&leave_request, &owner_id).unwrap();

  assert_eq!(state.room_count(), 0);
}

#[test]
fn test_total_member_count_after_room_destruction() {
  let state = RoomState::new();
  let owner_id = test_user_id(1);

  let (room_id, _) = state
    .create_room(&create_room_request(), owner_id.clone())
    .unwrap();
  assert_eq!(state.total_member_count(), 1);

  // Owner leaves - room destroyed, member count should be 0
  let leave_request = LeaveRoom { room_id };
  state.leave_room(&leave_request, &owner_id).unwrap();

  assert_eq!(state.total_member_count(), 0);
}
