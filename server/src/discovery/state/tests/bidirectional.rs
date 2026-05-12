//! Bidirectional conflict detection tests.

use super::*;

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
