//! Rate limiting tests.

use super::*;

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
