//! Single invitation management tests.

use super::*;

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
