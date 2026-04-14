//! Room state machine tests (MA-P2-002).
//! Tests for room lifecycle state transitions.

use super::*;

#[test]
fn test_room_state_creation() {
  let state = create_test_room_state();
  assert_eq!(state.room_count(), 0);
}

#[test]
fn test_room_count_increases_on_create() {
  let state = create_test_room_state();
  let owner_id = UserId::new();

  // Initially no rooms
  assert_eq!(state.room_count(), 0);

  // Create first room
  let create_request = CreateRoom {
    name: "Room 1".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  state
    .create_room(&create_request, owner_id.clone())
    .unwrap();
  assert_eq!(state.room_count(), 1);

  // Create second room with different owner
  let owner2_id = UserId::new();
  let create_request = CreateRoom {
    name: "Room 2".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  state.create_room(&create_request, owner2_id).unwrap();
  assert_eq!(state.room_count(), 2);
}

#[test]
fn test_room_count_decreases_on_destroy() {
  let state = create_test_room_state();
  let owner_id = UserId::new();

  // Create room
  let create_request = CreateRoom {
    name: "Test Room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = state
    .create_room(&create_request, owner_id.clone())
    .unwrap();
  assert_eq!(state.room_count(), 1);

  // Owner leaves (only member) - room gets destroyed
  let leave_request = LeaveRoom {
    room_id: room_id.clone(),
  };
  state.leave_room(&leave_request, &owner_id).unwrap();
  assert_eq!(state.room_count(), 0);
}

#[test]
fn test_multiple_rooms_independent_operations() {
  let state = create_test_room_state();
  let owner1_id = UserId::new();
  let owner2_id = UserId::new();
  let member1_id = UserId::new();
  let member2_id = UserId::new();

  // Create two rooms
  let create_request1 = CreateRoom {
    name: "Room 1".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room1_id, _) = state
    .create_room(&create_request1, owner1_id.clone())
    .unwrap();

  let create_request2 = CreateRoom {
    name: "Room 2".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room2_id, _) = state
    .create_room(&create_request2, owner2_id.clone())
    .unwrap();

  // Members join different rooms
  let join_request1 = JoinRoom {
    room_id: room1_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request1, member1_id.clone(), "Member1".to_string())
    .unwrap();

  let join_request2 = JoinRoom {
    room_id: room2_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request2, member2_id.clone(), "Member2".to_string())
    .unwrap();

  // Verify rooms are independent
  let room1 = state.get_room(&room1_id).unwrap();
  assert_eq!(room1.members.len(), 2);

  let room2 = state.get_room(&room2_id).unwrap();
  assert_eq!(room2.members.len(), 2);

  // Muting in room1 doesn't affect room2
  let mute_request = MuteMember {
    room_id: room1_id.clone(),
    target: member1_id.clone(),
    duration_secs: None,
  };
  state.mute_member(&mute_request, &owner1_id).unwrap();

  // Verify member1 is muted in room1 but member2 is not muted in room2
  let room1 = state.get_room(&room1_id).unwrap();
  assert!(room1.get_member(&member1_id).unwrap().is_muted());

  let room2 = state.get_room(&room2_id).unwrap();
  assert!(!room2.get_member(&member2_id).unwrap().is_muted());
}

#[test]
fn test_user_cannot_be_in_multiple_rooms() {
  let state = create_test_room_state();
  let owner1_id = UserId::new();
  let owner2_id = UserId::new();
  let common_user_id = UserId::new();

  // Create two rooms
  let create_request1 = CreateRoom {
    name: "Room 1".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room1_id, _) = state
    .create_room(&create_request1, owner1_id.clone())
    .unwrap();

  let create_request2 = CreateRoom {
    name: "Room 2".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room2_id, _) = state
    .create_room(&create_request2, owner2_id.clone())
    .unwrap();

  // User joins first room
  let join_request1 = JoinRoom {
    room_id: room1_id.clone(),
    password: None,
  };
  state
    .join_room(
      &join_request1,
      common_user_id.clone(),
      "CommonUser".to_string(),
    )
    .unwrap();

  // User tries to join second room (should fail - already in a room)
  let join_request2 = JoinRoom {
    room_id: room2_id.clone(),
    password: None,
  };
  let result = state.join_room(
    &join_request2,
    common_user_id.clone(),
    "CommonUser".to_string(),
  );
  assert_eq!(result.unwrap_err(), RoomError::UserAlreadyInRoom);

  // Verify user is only in first room
  let room1 = state.get_room(&room1_id).unwrap();
  assert!(room1.is_member(&common_user_id));

  let room2 = state.get_room(&room2_id).unwrap();
  assert!(!room2.is_member(&common_user_id));
}

#[test]
fn test_room_member_roles_independent_across_rooms() {
  let state = create_test_room_state();
  let owner1_id = UserId::new();
  let owner2_id = UserId::new();
  let user1_id = UserId::new();
  let user2_id = UserId::new();

  // Create two rooms
  let create_request1 = CreateRoom {
    name: "Room 1".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room1_id, _) = state
    .create_room(&create_request1, owner1_id.clone())
    .unwrap();

  let create_request2 = CreateRoom {
    name: "Room 2".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room2_id, _) = state
    .create_room(&create_request2, owner2_id.clone())
    .unwrap();

  // Different users join different rooms
  let join_request1 = JoinRoom {
    room_id: room1_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request1, user1_id.clone(), "User1".to_string())
    .unwrap();

  let join_request2 = JoinRoom {
    room_id: room2_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request2, user2_id.clone(), "User2".to_string())
    .unwrap();

  // Promote user1 to admin in room1 only
  let promote_request = PromoteAdmin {
    room_id: room1_id.clone(),
    target: user1_id.clone(),
  };
  state.promote_admin(&promote_request, &owner1_id).unwrap();

  // Verify roles
  let room1 = state.get_room(&room1_id).unwrap();
  assert_eq!(room1.get_member(&user1_id).unwrap().role, RoomRole::Admin);

  let room2 = state.get_room(&room2_id).unwrap();
  assert_eq!(room2.get_member(&user2_id).unwrap().role, RoomRole::Member);
}

#[test]
fn test_ban_state_persists_across_operations() {
  let state = create_test_room_state();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  // Create and join room
  let create_request = CreateRoom {
    name: "Test Room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = state
    .create_room(&create_request, owner_id.clone())
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Member".to_string())
    .unwrap();

  // Ban member
  let ban_request = BanMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
  };
  state.ban_member(&ban_request, &owner_id).unwrap();

  // Verify ban persists
  let room = state.get_room(&room_id).unwrap();
  assert!(room.is_banned(&member_id));

  // Try to join again (should still be banned)
  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  let result = state.join_room(&join_request, member_id.clone(), "Member".to_string());
  assert_eq!(result.unwrap_err(), RoomError::UserBanned);

  // After some time, ban should still persist
  let room = state.get_room(&room_id).unwrap();
  assert!(room.is_banned(&member_id));
}

#[test]
fn test_mute_state_persistence() {
  let state = create_test_room_state();
  let owner_id = UserId::new();
  let member_id = UserId::new();

  // Create and join room
  let create_request = CreateRoom {
    name: "Test Room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (room_id, _) = state
    .create_room(&create_request, owner_id.clone())
    .unwrap();

  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, member_id.clone(), "Member".to_string())
    .unwrap();

  // Mute permanently
  let mute_request = MuteMember {
    room_id: room_id.clone(),
    target: member_id.clone(),
    duration_secs: None,
  };
  state.mute_member(&mute_request, &owner_id).unwrap();

  // Verify mute persists across room queries
  let room = state.get_room(&room_id).unwrap();
  assert!(room.get_member(&member_id).unwrap().is_muted());

  // Perform other operations
  let other_member = UserId::new();
  let join_request = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  state
    .join_room(&join_request, other_member.clone(), "Other".to_string())
    .unwrap();

  // Mute should still be active
  let room = state.get_room(&room_id).unwrap();
  assert!(room.get_member(&member_id).unwrap().is_muted());
}

#[test]
fn test_room_list_operations() {
  let state = create_test_room_state();

  // Create multiple rooms with different owners
  for i in 0..5 {
    let owner_id = UserId::new();
    let create_request = CreateRoom {
      name: format!("Room {}", i),
      room_type: RoomType::Chat,
      password: None,
      max_participants: 8,
    };
    state.create_room(&create_request, owner_id).unwrap();
  }

  assert_eq!(state.room_count(), 5);

  // Verify all rooms exist
  let rooms = state.get_all_rooms();
  assert_eq!(rooms.len(), 5);
}

#[test]
fn test_concurrent_room_operations() {
  use std::sync::Arc;
  use std::thread;

  let state = Arc::new(std::sync::RwLock::new(create_test_room_state()));
  let mut handles = vec![];

  // Spawn multiple threads to create rooms concurrently
  for i in 0..10 {
    let state_clone = Arc::clone(&state);
    let handle = thread::spawn(move || {
      let owner_id = UserId::new();
      let create_request = CreateRoom {
        name: format!("Concurrent Room {}", i),
        room_type: RoomType::Chat,
        password: None,
        max_participants: 8,
      };
      let state = state_clone.write().unwrap();
      state.create_room(&create_request, owner_id)
    });
    handles.push(handle);
  }

  // Wait for all threads
  for handle in handles {
    let result = handle.join().unwrap();
    assert!(result.is_ok());
  }

  // Verify all rooms were created
  let state = state.read().unwrap();
  assert_eq!(state.room_count(), 10);
}
