//! Room handler integration tests

mod common;

use message::signal::{RoomType, SignalMessage};
use server::handler::room_handlers;

// ========================================================================
// Create Room
// ========================================================================

#[test]
fn test_create_room_success() {
  let state = common::new_state();
  let mut rx = common::register_mock_user(&state, "owner-1", "alice");

  room_handlers::handle_create_room(
    "owner-1",
    "Test Room".to_string(),
    Some("Description".to_string()),
    None,
    8,
    RoomType::Chat,
    &state,
  );

  let messages = common::drain_messages(&mut rx);

  // Should receive RoomCreated
  assert!(
    messages
      .iter()
      .any(|m| matches!(m, SignalMessage::RoomCreated { .. }))
  );

  // Should receive RoomMemberUpdate (owner themselves)
  assert!(
    messages
      .iter()
      .any(|m| matches!(m, SignalMessage::RoomMemberUpdate { members, .. } if members.len() == 1))
  );

  // Should receive RoomListUpdate (broadcast)
  assert!(
    messages
      .iter()
      .any(|m| matches!(m, SignalMessage::RoomListUpdate { rooms } if rooms.len() == 1))
  );
}

#[test]
fn test_create_room_with_password() {
  let state = common::new_state();
  let _rx = common::register_mock_user(&state, "owner-1", "alice");

  room_handlers::handle_create_room(
    "owner-1",
    "Password Room".to_string(),
    None,
    Some("secret123".to_string()),
    4,
    RoomType::Theater,
    &state,
  );

  let rooms = state.inner().rooms.list();
  assert_eq!(rooms.len(), 1);
  assert!(rooms[0].has_password);
  assert_eq!(rooms[0].room_type, RoomType::Theater);
}

// ========================================================================
// Join Room
// ========================================================================

/// Helper: create a room and return the room_id
fn setup_room(state: &server::state::AppState, owner_id: &str, password: Option<&str>) -> String {
  let password_hash = password.map(|p| server::auth::hash_password(p).unwrap());
  let room = message::room::Room::new(
    "Test Room".to_string(),
    None,
    password_hash,
    8,
    RoomType::Chat,
    owner_id.to_string(),
  );
  let room_id = room.id.clone();
  state.inner().rooms.insert(room);
  room_id
}

#[test]
fn test_join_room_success() {
  let state = common::new_state();
  let mut rx_owner = common::register_mock_user(&state, "owner-1", "alice");
  let mut rx_joiner = common::register_mock_user(&state, "user-2", "bob");

  let room_id = setup_room(&state, "owner-1", None);

  room_handlers::handle_join_room("user-2", room_id.clone(), None, &state);

  // Joiner should receive RoomMemberUpdate
  let msgs = common::drain_messages(&mut rx_joiner);
  assert!(
    msgs
      .iter()
      .any(|m| matches!(m, SignalMessage::RoomMemberUpdate { members, .. } if members.len() == 2))
  );

  // Owner should also receive RoomMemberUpdate
  let msgs = common::drain_messages(&mut rx_owner);
  assert!(
    msgs
      .iter()
      .any(|m| matches!(m, SignalMessage::RoomMemberUpdate { members, .. } if members.len() == 2))
  );
}

#[test]
fn test_join_room_nonexistent() {
  let state = common::new_state();
  let mut rx = common::register_mock_user(&state, "user-1", "alice");

  room_handlers::handle_join_room("user-1", "nonexistent".to_string(), None, &state);

  let msgs = common::drain_messages(&mut rx);
  assert!(msgs.iter().any(
    |m| matches!(m, SignalMessage::RoomError { reason } if reason.contains("does not exist"))
  ));
}

#[test]
fn test_join_room_already_member() {
  let state = common::new_state();
  let mut rx = common::register_mock_user(&state, "owner-1", "alice");

  let room_id = setup_room(&state, "owner-1", None);

  // Owner tries to join again
  room_handlers::handle_join_room("owner-1", room_id, None, &state);

  let msgs = common::drain_messages(&mut rx);
  assert!(
    msgs
      .iter()
      .any(|m| matches!(m, SignalMessage::RoomError { reason } if reason.contains("already in")))
  );
}

#[test]
fn test_join_room_full() {
  let state = common::new_state();
  let _rx_owner = common::register_mock_user(&state, "owner-1", "alice");

  // Create a room with max 1 member (owner already occupies 1 slot)
  let room = message::room::Room::new(
    "Small Room".to_string(),
    None,
    None,
    1,
    RoomType::Chat,
    "owner-1".to_string(),
  );
  let room_id = room.id.clone();
  state.inner().rooms.insert(room);

  let mut rx_joiner = common::register_mock_user(&state, "user-2", "bob");
  room_handlers::handle_join_room("user-2", room_id, None, &state);

  let msgs = common::drain_messages(&mut rx_joiner);
  assert!(
    msgs
      .iter()
      .any(|m| matches!(m, SignalMessage::RoomError { reason } if reason.contains("full")))
  );
}

#[test]
fn test_join_room_wrong_password() {
  let state = common::new_state();
  let _rx_owner = common::register_mock_user(&state, "owner-1", "alice");
  let mut rx_joiner = common::register_mock_user(&state, "user-2", "bob");

  let room_id = setup_room(&state, "owner-1", Some("correct_password"));

  room_handlers::handle_join_room(
    "user-2",
    room_id,
    Some("wrong_password".to_string()),
    &state,
  );

  let msgs = common::drain_messages(&mut rx_joiner);
  assert!(
    msgs
      .iter()
      .any(|m| matches!(m, SignalMessage::RoomError { reason } if reason.contains("password")))
  );
}

#[test]
fn test_join_room_correct_password() {
  let state = common::new_state();
  let _rx_owner = common::register_mock_user(&state, "owner-1", "alice");
  let mut rx_joiner = common::register_mock_user(&state, "user-2", "bob");

  let room_id = setup_room(&state, "owner-1", Some("correct_password"));

  room_handlers::handle_join_room(
    "user-2",
    room_id,
    Some("correct_password".to_string()),
    &state,
  );

  let msgs = common::drain_messages(&mut rx_joiner);
  assert!(
    msgs
      .iter()
      .any(|m| matches!(m, SignalMessage::RoomMemberUpdate { members, .. } if members.len() == 2))
  );
}

#[test]
fn test_join_room_blacklisted() {
  let state = common::new_state();
  let _rx_owner = common::register_mock_user(&state, "owner-1", "alice");
  let mut rx_joiner = common::register_mock_user(&state, "user-2", "bob");

  let room_id = setup_room(&state, "owner-1", None);

  // Add user-2 to blacklist
  if let Some(mut room) = state.inner().rooms.get_mut(&room_id) {
    room.blacklist.push("user-2".to_string());
  }

  room_handlers::handle_join_room("user-2", room_id, None, &state);

  let msgs = common::drain_messages(&mut rx_joiner);
  assert!(
    msgs
      .iter()
      .any(|m| matches!(m, SignalMessage::RoomError { reason } if reason.contains("kicked")))
  );
}

// ========================================================================
// Leave Room
// ========================================================================

#[test]
fn test_leave_room_member() {
  let state = common::new_state();
  let mut rx_owner = common::register_mock_user(&state, "owner-1", "alice");
  let _rx_joiner = common::register_mock_user(&state, "user-2", "bob");

  let room_id = setup_room(&state, "owner-1", None);

  // user-2 joins
  if let Some(mut room) = state.inner().rooms.get_mut(&room_id) {
    room.add_member("user-2".to_string()).unwrap();
  }

  // user-2 leaves
  room_handlers::handle_leave_room("user-2", room_id.clone(), &state);

  // Owner should receive updated member list (only 1 remaining)
  let msgs = common::drain_messages(&mut rx_owner);
  assert!(
    msgs
      .iter()
      .any(|m| matches!(m, SignalMessage::RoomMemberUpdate { members, .. } if members.len() == 1))
  );
}

#[test]
fn test_leave_room_owner_transfers() {
  let state = common::new_state();
  let _rx_owner = common::register_mock_user(&state, "owner-1", "alice");
  let mut rx_member = common::register_mock_user(&state, "user-2", "bob");

  let room_id = setup_room(&state, "owner-1", None);

  // user-2 joins
  if let Some(mut room) = state.inner().rooms.get_mut(&room_id) {
    room.add_member("user-2".to_string()).unwrap();
  }

  // Owner leaves → should auto-transfer ownership to user-2
  room_handlers::handle_leave_room("owner-1", room_id.clone(), &state);

  // Verify user-2 is now the owner
  if let Some(room) = state.inner().rooms.get(&room_id) {
    assert_eq!(room.owner_id, "user-2");
  }

  let msgs = common::drain_messages(&mut rx_member);
  assert!(
    msgs
      .iter()
      .any(|m| matches!(m, SignalMessage::RoomMemberUpdate { .. }))
  );
}

#[test]
fn test_leave_room_last_member_destroys() {
  let state = common::new_state();
  let _rx = common::register_mock_user(&state, "owner-1", "alice");

  let room_id = setup_room(&state, "owner-1", None);

  // Last member leaves → room should be destroyed
  room_handlers::handle_leave_room("owner-1", room_id.clone(), &state);

  assert!(state.inner().rooms.get(&room_id).is_none());
}

// ========================================================================
// Kick Member
// ========================================================================

#[test]
fn test_kick_member_success() {
  let state = common::new_state();
  let mut rx_owner = common::register_mock_user(&state, "owner-1", "alice");
  let mut rx_target = common::register_mock_user(&state, "user-2", "bob");

  let room_id = setup_room(&state, "owner-1", None);
  if let Some(mut room) = state.inner().rooms.get_mut(&room_id) {
    room.add_member("user-2".to_string()).unwrap();
  }

  room_handlers::handle_kick_member("owner-1", room_id.clone(), "user-2".to_string(), &state);

  // Kicked user should receive Kicked message
  let msgs = common::drain_messages(&mut rx_target);
  assert!(
    msgs
      .iter()
      .any(|m| matches!(m, SignalMessage::Kicked { .. }))
  );

  // Owner should receive updated member list
  let msgs = common::drain_messages(&mut rx_owner);
  assert!(
    msgs
      .iter()
      .any(|m| matches!(m, SignalMessage::RoomMemberUpdate { members, .. } if members.len() == 1))
  );

  // Kicked user should be on the blacklist
  if let Some(room) = state.inner().rooms.get(&room_id) {
    assert!(room.is_blacklisted("user-2"));
  }
}

#[test]
fn test_kick_member_not_owner() {
  let state = common::new_state();
  let _rx_owner = common::register_mock_user(&state, "owner-1", "alice");
  let mut rx_member = common::register_mock_user(&state, "user-2", "bob");

  let room_id = setup_room(&state, "owner-1", None);
  if let Some(mut room) = state.inner().rooms.get_mut(&room_id) {
    room.add_member("user-2".to_string()).unwrap();
  }

  // Non-owner tries to kick
  room_handlers::handle_kick_member("user-2", room_id, "owner-1".to_string(), &state);

  let msgs = common::drain_messages(&mut rx_member);
  assert!(
    msgs
      .iter()
      .any(|m| matches!(m, SignalMessage::RoomError { reason } if reason.contains("owner")))
  );
}

// ========================================================================
// Mute
// ========================================================================

#[test]
fn test_mute_member_success() {
  let state = common::new_state();
  let _rx_owner = common::register_mock_user(&state, "owner-1", "alice");
  let mut rx_target = common::register_mock_user(&state, "user-2", "bob");

  let room_id = setup_room(&state, "owner-1", None);
  if let Some(mut room) = state.inner().rooms.get_mut(&room_id) {
    room.add_member("user-2".to_string()).unwrap();
  }

  room_handlers::handle_mute_member(
    "owner-1",
    room_id.clone(),
    "user-2".to_string(),
    true,
    &state,
  );

  let msgs = common::drain_messages(&mut rx_target);
  assert!(
    msgs
      .iter()
      .any(|m| matches!(m, SignalMessage::MuteStatusChanged { muted: true, .. }))
  );
}

#[test]
fn test_mute_all_success() {
  let state = common::new_state();
  let mut rx_owner = common::register_mock_user(&state, "owner-1", "alice");
  let mut rx_member = common::register_mock_user(&state, "user-2", "bob");

  let room_id = setup_room(&state, "owner-1", None);
  if let Some(mut room) = state.inner().rooms.get_mut(&room_id) {
    room.add_member("user-2".to_string()).unwrap();
  }

  room_handlers::handle_mute_all("owner-1", room_id.clone(), true, &state);

  // All members should receive MuteStatusChanged
  let msgs_owner = common::drain_messages(&mut rx_owner);
  assert!(
    msgs_owner
      .iter()
      .any(|m| matches!(m, SignalMessage::MuteStatusChanged { muted: true, .. }))
  );

  let msgs_member = common::drain_messages(&mut rx_member);
  assert!(
    msgs_member
      .iter()
      .any(|m| matches!(m, SignalMessage::MuteStatusChanged { muted: true, .. }))
  );
}

// ========================================================================
// Transfer Ownership
// ========================================================================

#[test]
fn test_transfer_owner_success() {
  let state = common::new_state();
  let _rx_owner = common::register_mock_user(&state, "owner-1", "alice");
  let mut rx_member = common::register_mock_user(&state, "user-2", "bob");

  let room_id = setup_room(&state, "owner-1", None);
  if let Some(mut room) = state.inner().rooms.get_mut(&room_id) {
    room.add_member("user-2".to_string()).unwrap();
  }

  room_handlers::handle_transfer_owner("owner-1", room_id.clone(), "user-2".to_string(), &state);

  // Verify new owner
  if let Some(room) = state.inner().rooms.get(&room_id) {
    assert_eq!(room.owner_id, "user-2");
  }

  let msgs = common::drain_messages(&mut rx_member);
  assert!(
    msgs
      .iter()
      .any(|m| matches!(m, SignalMessage::RoomMemberUpdate { .. }))
  );
}

#[test]
fn test_transfer_owner_not_owner() {
  let state = common::new_state();
  let _rx_owner = common::register_mock_user(&state, "owner-1", "alice");
  let mut rx_member = common::register_mock_user(&state, "user-2", "bob");

  let room_id = setup_room(&state, "owner-1", None);
  if let Some(mut room) = state.inner().rooms.get_mut(&room_id) {
    room.add_member("user-2".to_string()).unwrap();
  }

  // Non-owner tries to transfer
  room_handlers::handle_transfer_owner("user-2", room_id, "owner-1".to_string(), &state);

  let msgs = common::drain_messages(&mut rx_member);
  assert!(
    msgs
      .iter()
      .any(|m| matches!(m, SignalMessage::RoomError { reason } if reason.contains("owner")))
  );
}

// ========================================================================
// Theater
// ========================================================================

#[test]
fn test_theater_control_by_owner() {
  let state = common::new_state();
  let _rx_owner = common::register_mock_user(&state, "owner-1", "alice");
  let mut rx_member = common::register_mock_user(&state, "user-2", "bob");

  let room_id = setup_room(&state, "owner-1", None);
  if let Some(mut room) = state.inner().rooms.get_mut(&room_id) {
    room.add_member("user-2".to_string()).unwrap();
  }

  room_handlers::handle_theater_control(
    "owner-1",
    room_id,
    message::signal::TheaterAction::Play,
    &state,
  );

  // Member should receive TheaterControl
  let msgs = common::drain_messages(&mut rx_member);
  assert!(msgs.iter().any(|m| matches!(m, SignalMessage::TheaterControl { action, .. } if *action == message::signal::TheaterAction::Play)));
}

#[test]
fn test_theater_sync() {
  let state = common::new_state();
  let _rx_owner = common::register_mock_user(&state, "owner-1", "alice");
  let mut rx_member = common::register_mock_user(&state, "user-2", "bob");

  let room_id = setup_room(&state, "owner-1", None);
  if let Some(mut room) = state.inner().rooms.get_mut(&room_id) {
    room.add_member("user-2".to_string()).unwrap();
  }

  room_handlers::handle_theater_sync("owner-1", room_id, 120.5, true, &state);

  let msgs = common::drain_messages(&mut rx_member);
  assert!(msgs.iter().any(|m| matches!(m, SignalMessage::TheaterSync { current_time, is_playing: true, .. } if (*current_time - 120.5).abs() < f64::EPSILON)));
}
