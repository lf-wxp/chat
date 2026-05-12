//! Multi-invite management tests.

use super::*;

#[test]
fn test_send_multi_invitation_success() {
  let state = DiscoveryState::new();
  let from = UserId::new();
  let to1 = UserId::new();
  let to2 = UserId::new();

  let invite = create_multi_invite(from.clone(), vec![to1.clone(), to2.clone()]);
  let result = state.send_multi_invitation(&invite);
  assert!(result.is_ok());

  // Both should have pending invitations
  assert!(state.has_pending_invitation(&from, &to1));
  assert!(state.has_pending_invitation(&from, &to2));
}

#[test]
fn test_send_multi_invitation_no_valid_targets() {
  let state = DiscoveryState::new();
  let from = UserId::new();
  let to1 = UserId::new();

  // First invite fills the target
  state
    .send_invitation(&create_invite(from.clone(), to1.clone()))
    .unwrap();

  // Multi-invite with same target should have no valid targets (already pending)
  let invite = create_multi_invite(from, vec![to1]);
  let result = state.send_multi_invitation(&invite);
  assert_eq!(result.unwrap_err(), InvitationError::NoValidTargets);
}

#[test]
fn test_accept_multi_invitation_first_acceptance() {
  let state = DiscoveryState::new();
  let from = UserId::new();
  let to1 = UserId::new();
  let to2 = UserId::new();
  let room_id = RoomId::new();

  let invite = create_multi_invite(from.clone(), vec![to1.clone(), to2.clone()]);
  state.send_multi_invitation(&invite).unwrap();

  // First acceptance should return FirstAcceptance
  let result = state.accept_multi_invitation(&from, &to1, room_id.clone());
  assert!(result.is_some());

  match result.unwrap() {
    MultiInviteAcceptResult::FirstAcceptance {
      room_id: rid,
      remaining_targets,
    } => {
      assert_eq!(rid, room_id);
      assert!(!remaining_targets.is_empty());
    }
    MultiInviteAcceptResult::JoinRoom { .. } => {
      panic!("Expected FirstAcceptance");
    }
  }

  // Pending invitation should be removed
  assert!(!state.has_pending_invitation(&from, &to1));
}

#[test]
fn test_accept_multi_invitation_join_room() {
  let state = DiscoveryState::new();
  let from = UserId::new();
  let to1 = UserId::new();
  let to2 = UserId::new();
  let room_id = RoomId::new();

  let invite = create_multi_invite(from.clone(), vec![to1.clone(), to2.clone()]);
  state.send_multi_invitation(&invite).unwrap();

  // First acceptance
  state.accept_multi_invitation(&from, &to1, room_id.clone());

  // Second acceptance should return JoinRoom
  let result = state.accept_multi_invitation(&from, &to2, room_id.clone());
  assert!(result.is_some());

  match result.unwrap() {
    MultiInviteAcceptResult::JoinRoom { room_id: rid } => {
      assert_eq!(rid, room_id);
    }
    MultiInviteAcceptResult::FirstAcceptance { .. } => {
      panic!("Expected JoinRoom");
    }
  }
}

#[test]
fn test_decline_multi_invitation() {
  let state = DiscoveryState::new();
  let from = UserId::new();
  let to1 = UserId::new();
  let to2 = UserId::new();

  let invite = create_multi_invite(from.clone(), vec![to1.clone(), to2.clone()]);
  state.send_multi_invitation(&invite).unwrap();

  state.decline_multi_invitation(&from, &to1);
  assert!(!state.has_pending_invitation(&from, &to1));
  assert!(state.has_pending_invitation(&from, &to2));
}

#[test]
fn test_get_multi_invite_stats() {
  let state = DiscoveryState::new();
  let from = UserId::new();
  let to1 = UserId::new();
  let to2 = UserId::new();
  let room_id = RoomId::new();

  let invite = create_multi_invite(from.clone(), vec![to1.clone(), to2.clone()]);
  let inv_id = state.send_multi_invitation(&invite).unwrap();

  state.accept_multi_invitation(&from, &to1, room_id);

  let stats = state.get_multi_invite_stats(&inv_id);
  assert!(stats.is_some());
  let stats = stats.unwrap();
  assert_eq!(stats.total_targets, 2);
  assert_eq!(stats.accepted, 1);
  assert_eq!(stats.declined, 0);
}

#[test]
fn test_is_multi_invite_complete_false_when_pending() {
  let state = DiscoveryState::new();
  let from = UserId::new();
  let to1 = UserId::new();
  let to2 = UserId::new();

  let invite = create_multi_invite(from, vec![to1, to2]);
  let inv_id = state.send_multi_invitation(&invite).unwrap();

  assert!(!state.is_multi_invite_complete(&inv_id));
}
