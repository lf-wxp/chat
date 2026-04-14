use super::*;
use crate::discovery::{INVITE_RATE_LIMIT_PER_HOUR, INVITE_RATE_LIMIT_PER_MINUTE};

fn create_invite(from: UserId, to: UserId) -> ConnectionInvite {
  ConnectionInvite {
    from: from.clone(),
    to: to.clone(),
    note: Some("Let's chat!".to_string()),
  }
}

fn create_multi_invite(from: UserId, targets: Vec<UserId>) -> MultiInvite {
  MultiInvite { from, targets }
}

// ===========================================================================
// DiscoveryState creation
// ===========================================================================

#[test]
fn test_new_discovery_state_is_empty() {
  let state = DiscoveryState::new();
  let user = UserId::new();
  assert_eq!(state.pending_invitation_count(&user), 0);
  assert!(state.get_active_peers(&user).is_empty());
  assert!(state.get_pending_sdp_negotiations(&user).is_empty());
}

#[test]
fn test_default_discovery_state() {
  let state = DiscoveryState::default();
  let user = UserId::new();
  assert_eq!(state.pending_invitation_count(&user), 0);
}

// ===========================================================================
// Rate limiting via DiscoveryState
// ===========================================================================

#[test]
fn test_can_send_invitation_initially() {
  let state = DiscoveryState::new();
  let user = UserId::new();
  assert!(state.can_send_invitation(&user));
}

#[test]
fn test_get_remaining_quota_initially_full() {
  let state = DiscoveryState::new();
  let user = UserId::new();
  let (minute, hour) = state.get_remaining_quota(&user);
  assert_eq!(minute, INVITE_RATE_LIMIT_PER_MINUTE);
  assert_eq!(hour, INVITE_RATE_LIMIT_PER_HOUR);
}

// ===========================================================================
// Single invitation management
// ===========================================================================

#[test]
fn test_send_invitation_success() {
  let state = DiscoveryState::new();
  let from = UserId::new();
  let to = UserId::new();
  let invite = create_invite(from.clone(), to.clone());

  let result = state.send_invitation(&invite);
  assert!(result.is_ok());
  assert!(state.has_pending_invitation(&from, &to));
}

#[test]
fn test_send_invitation_duplicate_rejected() {
  let state = DiscoveryState::new();
  let from = UserId::new();
  let to = UserId::new();
  let invite = create_invite(from.clone(), to.clone());

  state.send_invitation(&invite).unwrap();
  let result = state.send_invitation(&invite);
  assert_eq!(result.unwrap_err(), InvitationError::AlreadyPending);
}

#[test]
fn test_accept_invitation_success() {
  let state = DiscoveryState::new();
  let from = UserId::new();
  let to = UserId::new();
  let invite = create_invite(from.clone(), to.clone());

  state.send_invitation(&invite).unwrap();
  let accepted = state.accept_invitation(&from, &to);
  assert!(accepted.is_some());
  assert!(!state.has_pending_invitation(&from, &to));
}

#[test]
fn test_accept_invitation_not_found() {
  let state = DiscoveryState::new();
  let from = UserId::new();
  let to = UserId::new();

  let accepted = state.accept_invitation(&from, &to);
  assert!(accepted.is_none());
}

#[test]
fn test_decline_invitation_success() {
  let state = DiscoveryState::new();
  let from = UserId::new();
  let to = UserId::new();
  let invite = create_invite(from.clone(), to.clone());

  state.send_invitation(&invite).unwrap();
  let declined = state.decline_invitation(&from, &to);
  assert!(declined.is_some());
  assert!(!state.has_pending_invitation(&from, &to));
}

#[test]
fn test_get_pending_sent() {
  let state = DiscoveryState::new();
  let from = UserId::new();
  let to1 = UserId::new();
  let to2 = UserId::new();

  state
    .send_invitation(&create_invite(from.clone(), to1))
    .unwrap();
  state
    .send_invitation(&create_invite(from.clone(), to2))
    .unwrap();

  let sent = state.get_pending_sent(&from);
  assert_eq!(sent.len(), 2);
}

#[test]
fn test_get_pending_received() {
  let state = DiscoveryState::new();
  let from1 = UserId::new();
  let from2 = UserId::new();
  let to = UserId::new();

  state
    .send_invitation(&create_invite(from1, to.clone()))
    .unwrap();
  state
    .send_invitation(&create_invite(from2, to.clone()))
    .unwrap();

  let received = state.get_pending_received(&to);
  assert_eq!(received.len(), 2);
}

#[test]
fn test_pending_invitation_count() {
  let state = DiscoveryState::new();
  let from = UserId::new();
  let to = UserId::new();

  state
    .send_invitation(&create_invite(from.clone(), to.clone()))
    .unwrap();
  // from has 1 sent, to has 1 received
  assert_eq!(state.pending_invitation_count(&from), 1);
  assert_eq!(state.pending_invitation_count(&to), 1);
}

// ===========================================================================
// Bidirectional conflict detection
// ===========================================================================

#[test]
fn test_check_bidirectional_conflict_exists() {
  let state = DiscoveryState::new();
  let user_a = UserId::new();
  let user_b = UserId::new();

  // A invites B
  state
    .send_invitation(&create_invite(user_a.clone(), user_b.clone()))
    .unwrap();

  // Check if B has a pending invitation to A (should be None)
  assert!(
    state
      .check_bidirectional_conflict(&user_b, &user_a)
      .is_some()
  );
}

#[test]
fn test_check_bidirectional_conflict_none() {
  let state = DiscoveryState::new();
  let user_a = UserId::new();
  let user_b = UserId::new();

  assert!(
    state
      .check_bidirectional_conflict(&user_a, &user_b)
      .is_none()
  );
}

#[test]
fn test_merge_bidirectional_invitations() {
  let state = DiscoveryState::new();
  let user_a = UserId::new();
  let user_b = UserId::new();

  // Both users invite each other
  state
    .send_invitation(&create_invite(user_a.clone(), user_b.clone()))
    .unwrap();
  state
    .send_invitation(&create_invite(user_b.clone(), user_a.clone()))
    .unwrap();

  // Merge should succeed
  let result = state.merge_bidirectional_invitations(&user_a, &user_b);
  assert!(result.is_some());

  // Both invitations should be removed
  assert!(!state.has_pending_invitation(&user_a, &user_b));
  assert!(!state.has_pending_invitation(&user_b, &user_a));
}

#[test]
fn test_merge_bidirectional_invitations_no_reverse() {
  let state = DiscoveryState::new();
  let user_a = UserId::new();
  let user_b = UserId::new();

  // Only A invites B
  state
    .send_invitation(&create_invite(user_a.clone(), user_b.clone()))
    .unwrap();

  // Merge should fail without both directions
  let result = state.merge_bidirectional_invitations(&user_a, &user_b);
  assert!(result.is_none());
}

// ===========================================================================
// Target limit (unanswered invitations per target)
// ===========================================================================

#[test]
fn test_target_limit_exceeded() {
  let state = DiscoveryState::new();
  let to = UserId::new();

  // Send invitations from different users up to the limit
  for _ in 0..MAX_UNANSWERED_INVITATIONS_PER_TARGET {
    let from = UserId::new();
    state
      .send_invitation(&create_invite(from, to.clone()))
      .unwrap();
  }

  // One more should be rejected
  let extra_from = UserId::new();
  let result = state.send_invitation(&create_invite(extra_from, to));
  assert_eq!(result.unwrap_err(), InvitationError::TargetLimitExceeded);
}

#[test]
fn test_target_limit_decreased_on_accept() {
  let state = DiscoveryState::new();
  let to = UserId::new();
  let mut from_ids = Vec::new();

  // Send invitations up to the limit
  for _ in 0..MAX_UNANSWERED_INVITATIONS_PER_TARGET {
    let from = UserId::new();
    from_ids.push(from.clone());
    state
      .send_invitation(&create_invite(from, to.clone()))
      .unwrap();
  }

  // Accept one invitation
  state.accept_invitation(&from_ids[0], &to);

  // Now one more should be allowed
  let new_from = UserId::new();
  let result = state.send_invitation(&create_invite(new_from, to));
  assert!(result.is_ok());
}

// ===========================================================================
// Clear pending invitations for user (disconnect)
// ===========================================================================

#[test]
fn test_clear_pending_invitations_for_user() {
  let state = DiscoveryState::new();
  let user_a = UserId::new();
  let user_b = UserId::new();
  let user_c = UserId::new();

  // A invites B, C invites A
  state
    .send_invitation(&create_invite(user_a.clone(), user_b.clone()))
    .unwrap();
  state
    .send_invitation(&create_invite(user_c.clone(), user_a.clone()))
    .unwrap();

  // Clear all for user_a
  let removed = state.clear_pending_invitations_for_user(&user_a);
  assert_eq!(removed.len(), 2);
  assert!(!state.has_pending_invitation(&user_a, &user_b));
  assert!(!state.has_pending_invitation(&user_c, &user_a));
}

// ===========================================================================
// Active Peers Management
// ===========================================================================

#[test]
fn test_add_active_peer_bidirectional() {
  let state = DiscoveryState::new();
  let user_a = UserId::new();
  let user_b = UserId::new();

  state.add_active_peer(&user_a, &user_b);

  assert!(state.are_peers(&user_a, &user_b));
  assert!(state.are_peers(&user_b, &user_a));
}

#[test]
fn test_remove_active_peer() {
  let state = DiscoveryState::new();
  let user_a = UserId::new();
  let user_b = UserId::new();

  state.add_active_peer(&user_a, &user_b);
  state.remove_active_peer(&user_a, &user_b);

  assert!(!state.are_peers(&user_a, &user_b));
  assert!(!state.are_peers(&user_b, &user_a));
}

#[test]
fn test_get_active_peers_multiple() {
  let state = DiscoveryState::new();
  let user_a = UserId::new();
  let user_b = UserId::new();
  let user_c = UserId::new();

  state.add_active_peer(&user_a, &user_b);
  state.add_active_peer(&user_a, &user_c);

  let peers = state.get_active_peers(&user_a);
  assert_eq!(peers.len(), 2);
  assert!(peers.contains(&user_b));
  assert!(peers.contains(&user_c));
}

#[test]
fn test_are_peers_false_when_no_relationship() {
  let state = DiscoveryState::new();
  let user_a = UserId::new();
  let user_b = UserId::new();

  assert!(!state.are_peers(&user_a, &user_b));
}

#[test]
fn test_clear_active_peers() {
  let state = DiscoveryState::new();
  let user_a = UserId::new();
  let user_b = UserId::new();
  let user_c = UserId::new();

  state.add_active_peer(&user_a, &user_b);
  state.add_active_peer(&user_a, &user_c);

  state.clear_active_peers(&user_a);

  assert!(state.get_active_peers(&user_a).is_empty());
  assert!(!state.are_peers(&user_b, &user_a));
  assert!(!state.are_peers(&user_c, &user_a));
}

// ===========================================================================
// SDP Negotiation Management
// ===========================================================================

#[test]
fn test_start_sdp_negotiation() {
  let state = DiscoveryState::new();
  let from = UserId::new();
  let to = UserId::new();

  let started = state.start_sdp_negotiation(&from, &to);
  assert!(started);
  assert!(state.is_sdp_negotiation_in_progress(&from, &to));
}

#[test]
fn test_start_sdp_negotiation_already_in_progress() {
  let state = DiscoveryState::new();
  let from = UserId::new();
  let to = UserId::new();

  state.start_sdp_negotiation(&from, &to);
  let second = state.start_sdp_negotiation(&from, &to);
  assert!(!second);
}

#[test]
fn test_mark_offer_sent_and_answer_received() {
  let state = DiscoveryState::new();
  let from = UserId::new();
  let to = UserId::new();

  state.start_sdp_negotiation(&from, &to);
  state.mark_offer_sent(&from, &to);
  state.mark_answer_received(&to, &from);

  // Negotiation should still be in progress until completed
  assert!(state.is_sdp_negotiation_in_progress(&from, &to));
}

#[test]
fn test_complete_sdp_negotiation() {
  let state = DiscoveryState::new();
  let from = UserId::new();
  let to = UserId::new();

  state.start_sdp_negotiation(&from, &to);
  state.complete_sdp_negotiation(&from, &to);

  assert!(!state.is_sdp_negotiation_in_progress(&from, &to));
}

#[test]
fn test_get_pending_sdp_negotiations() {
  let state = DiscoveryState::new();
  let user_a = UserId::new();
  let user_b = UserId::new();

  state.start_sdp_negotiation(&user_a, &user_b);

  let pending = state.get_pending_sdp_negotiations(&user_a);
  assert_eq!(pending.len(), 1);

  let pending_b = state.get_pending_sdp_negotiations(&user_b);
  assert_eq!(pending_b.len(), 1);
}

#[test]
fn test_clear_sdp_negotiations_for_user() {
  let state = DiscoveryState::new();
  let user_a = UserId::new();
  let user_b = UserId::new();
  let user_c = UserId::new();

  state.start_sdp_negotiation(&user_a, &user_b);
  state.start_sdp_negotiation(&user_a, &user_c);

  state.clear_sdp_negotiations_for_user(&user_a);

  assert!(state.get_pending_sdp_negotiations(&user_a).is_empty());
  assert!(state.get_pending_sdp_negotiations(&user_b).is_empty());
  assert!(state.get_pending_sdp_negotiations(&user_c).is_empty());
}

// ===========================================================================
// Multi-Invite Management
// ===========================================================================

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
