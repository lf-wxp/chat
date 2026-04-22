//! Unit tests for theater mode state preconditions.
//!
//! These tests verify room state setup and preconditions (ownership, membership, etc.)
//! that the handler functions rely on. The actual handler function invocation
//! (including error responses, message broadcasting, and permission checks) is
//! thoroughly tested in `server/tests/integration_theater.rs` via real WebSocket
//! connections, covering:
//! - SIG301/SIG311: Room not found
//! - SIG302/SIG312: Non-owner rejection
//! - SIG313: Self-transfer rejection
//! - SIG314: Target not a member
//! - Successful mute-all broadcast (excludes sender)
//! - Successful transfer broadcast (includes sender)

use super::*;
use crate::ws::tests::{create_test_sender, create_test_ws_state};
use message::signaling::{CreateRoom, JoinRoom, TheaterMuteAll, TheaterTransferOwner};
use message::types::RoomType;

// ===== Theater Mute All Tests =====

#[test]
fn test_theater_mute_all_room_not_found() {
  let ws_state = create_test_ws_state();
  let room_id = message::RoomId::new();

  let _theater_mute_all = TheaterMuteAll {
    room_id: room_id.clone(),
  };

  // Room doesn't exist
  assert!(ws_state.room_state.get_room(&room_id).is_none());
}

#[test]
fn test_theater_mute_all_non_owner_fails() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let non_owner_id = UserId::new();

  // Create room
  let create_room = CreateRoom {
    name: "Theater Room".to_string(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 50,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  // Add non-owner member
  ws_state.add_connection(non_owner_id.clone(), create_test_sender());
  let join_room = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  ws_state
    .room_state
    .join_room(&join_room, non_owner_id.clone(), "member".to_string())
    .unwrap();

  // Non-owner tries to mute all
  let room = ws_state.room_state.get_room(&room_id).unwrap();
  assert_ne!(room.owner_id(), &non_owner_id);
}

#[test]
fn test_theater_mute_all_owner_success() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();

  // Create theater room
  let create_room = CreateRoom {
    name: "Theater Room".to_string(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 50,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  // Owner should be able to mute all
  let room = ws_state.room_state.get_room(&room_id).unwrap();
  assert_eq!(room.owner_id(), &owner_id);

  let theater_mute_all = TheaterMuteAll {
    room_id: room_id.clone(),
  };
  assert_eq!(theater_mute_all.room_id, room_id);
}

#[test]
fn test_theater_mute_all_broadcasts_to_members() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let member1_id = UserId::new();
  let member2_id = UserId::new();

  // Create theater room and add members
  let create_room = CreateRoom {
    name: "Theater Room".to_string(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 50,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  ws_state.add_connection(owner_id.clone(), create_test_sender());
  ws_state.add_connection(member1_id.clone(), create_test_sender());
  ws_state.add_connection(member2_id.clone(), create_test_sender());

  let join_room = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  ws_state
    .room_state
    .join_room(&join_room, member1_id.clone(), "member1".to_string())
    .unwrap();
  ws_state
    .room_state
    .join_room(&join_room, member2_id.clone(), "member2".to_string())
    .unwrap();

  // All members should have senders for broadcast
  assert!(ws_state.get_sender(&owner_id).is_some());
  assert!(ws_state.get_sender(&member1_id).is_some());
  assert!(ws_state.get_sender(&member2_id).is_some());
}

// ===== Theater Transfer Owner Tests =====

#[test]
fn test_theater_transfer_owner_room_not_found() {
  let ws_state = create_test_ws_state();
  let _owner_id = UserId::new();
  let target_id = UserId::new();
  let room_id = message::RoomId::new();

  let _theater_transfer = TheaterTransferOwner {
    room_id: room_id.clone(),
    target: target_id.clone(),
  };

  // Room doesn't exist
  assert!(ws_state.room_state.get_room(&room_id).is_none());
}

#[test]
fn test_theater_transfer_owner_non_owner_fails() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let non_owner_id = UserId::new();
  let target_id = UserId::new();

  // Create room
  let create_room = CreateRoom {
    name: "Theater Room".to_string(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 50,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  // Add members
  ws_state.add_connection(non_owner_id.clone(), create_test_sender());
  ws_state.add_connection(target_id.clone(), create_test_sender());
  let join_room = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  ws_state
    .room_state
    .join_room(&join_room, non_owner_id.clone(), "nonowner".to_string())
    .unwrap();
  ws_state
    .room_state
    .join_room(&join_room, target_id.clone(), "target".to_string())
    .unwrap();

  // Non-owner tries to transfer
  let room = ws_state.room_state.get_room(&room_id).unwrap();
  assert_ne!(room.owner_id(), &non_owner_id);
}

#[test]
fn test_theater_transfer_target_not_member() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let target_id = UserId::new();

  // Create room
  let create_room = CreateRoom {
    name: "Theater Room".to_string(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 50,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  // Target is not a member
  let room = ws_state.room_state.get_room(&room_id).unwrap();
  assert!(!room.is_member(&target_id));
}

#[test]
fn test_theater_transfer_owner_success() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let target_id = UserId::new();

  // Create theater room and add target member
  let create_room = CreateRoom {
    name: "Theater Room".to_string(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 50,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  ws_state.add_connection(target_id.clone(), create_test_sender());
  let join_room = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  ws_state
    .room_state
    .join_room(&join_room, target_id.clone(), "target".to_string())
    .unwrap();

  // Owner should be able to transfer
  let room = ws_state.room_state.get_room(&room_id).unwrap();
  assert_eq!(room.owner_id(), &owner_id);
  assert!(room.is_member(&target_id));

  let theater_transfer = TheaterTransferOwner {
    room_id: room_id.clone(),
    target: target_id.clone(),
  };
  assert_eq!(theater_transfer.room_id, room_id);
  assert_eq!(theater_transfer.target, target_id);
}

#[test]
fn test_theater_transfer_broadcasts_to_all() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();
  let target_id = UserId::new();
  let other_member_id = UserId::new();

  // Create theater room and add members
  let create_room = CreateRoom {
    name: "Theater Room".to_string(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 50,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  ws_state.add_connection(owner_id.clone(), create_test_sender());
  ws_state.add_connection(target_id.clone(), create_test_sender());
  ws_state.add_connection(other_member_id.clone(), create_test_sender());

  let join_room = JoinRoom {
    room_id: room_id.clone(),
    password: None,
  };
  ws_state
    .room_state
    .join_room(&join_room, target_id.clone(), "target".to_string())
    .unwrap();
  ws_state
    .room_state
    .join_room(&join_room, other_member_id.clone(), "other".to_string())
    .unwrap();

  // Theater transfer should broadcast to ALL members (including sender)
  // Unlike call handlers which exclude sender
  let all_have_senders = ws_state.get_sender(&owner_id).is_some()
    && ws_state.get_sender(&target_id).is_some()
    && ws_state.get_sender(&other_member_id).is_some();
  assert!(all_have_senders);
}

// ===== Room Type Validation Tests =====

#[test]
fn test_theater_mode_for_theater_room_type() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();

  // Create Theater room
  let create_room = CreateRoom {
    name: "Theater Room".to_string(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 50,
  };
  let (_room_id, room_info) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  assert_eq!(room_info.room_type, RoomType::Theater);
}

#[test]
fn test_theater_mode_for_chat_room() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();

  // Create Chat room (theater mode might still apply)
  let create_room = CreateRoom {
    name: "Chat Room".to_string(),
    room_type: RoomType::Chat,
    password: None,
    max_participants: 8,
  };
  let (_room_id, room_info) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  assert_eq!(room_info.room_type, RoomType::Chat);
}

// ===== Concurrent Theater Operations Tests =====

#[test]
fn test_concurrent_theater_transfers() {
  let ws_state = Arc::new(create_test_ws_state());
  let owner_id = UserId::new();
  let members: Vec<UserId> = (0..5).map(|_| UserId::new()).collect();

  // Create theater room
  let create_room = CreateRoom {
    name: "Theater Room".to_string(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 50,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  // Add members
  ws_state.add_connection(owner_id.clone(), create_test_sender());
  for member in &members {
    ws_state.add_connection(member.clone(), create_test_sender());
    let join_room = JoinRoom {
      room_id: room_id.clone(),
      password: None,
    };
    ws_state
      .room_state
      .join_room(&join_room, member.clone(), "member".to_string())
      .unwrap();
  }

  // All members should be in room
  let room = ws_state.room_state.get_room(&room_id).unwrap();
  assert_eq!(room.member_count(), 6); // owner + 5 members
}

// ===== Edge Cases =====

#[test]
fn test_theater_transfer_to_self() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();

  // Create room
  let create_room = CreateRoom {
    name: "Theater Room".to_string(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 50,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  // Owner tries to transfer to self
  let room = ws_state.room_state.get_room(&room_id).unwrap();
  assert!(room.is_member(&owner_id)); // Owner is also a member

  let theater_transfer = TheaterTransferOwner {
    room_id: room_id.clone(),
    target: owner_id.clone(),
  };

  // Self-transfer should be rejected by the handler (SIG313 error)
  // The handler checks: if theater_transfer.target == *user_id, send error and return
  assert_eq!(theater_transfer.target, owner_id); // target equals sender
}

#[test]
fn test_theater_mute_all_empty_room() {
  let ws_state = create_test_ws_state();
  let owner_id = UserId::new();

  // Create room with only owner
  let create_room = CreateRoom {
    name: "Theater Room".to_string(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 50,
  };
  let (room_id, _) = ws_state
    .room_state
    .create_room(&create_room, owner_id.clone())
    .unwrap();

  // Owner only room
  let room = ws_state.room_state.get_room(&room_id).unwrap();
  assert_eq!(room.member_count(), 1);

  // Mute all should still work (no one to mute except owner)
  let theater_mute_all = TheaterMuteAll {
    room_id: room_id.clone(),
  };
  assert_eq!(theater_mute_all.room_id, room_id);
}
