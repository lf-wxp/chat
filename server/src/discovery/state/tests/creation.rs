//! DiscoveryState creation and default tests.

use super::*;

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
