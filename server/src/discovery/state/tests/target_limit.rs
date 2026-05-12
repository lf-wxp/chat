//! Target limit tests.

use super::*;

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
