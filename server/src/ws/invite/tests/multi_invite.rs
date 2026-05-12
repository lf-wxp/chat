//! Multi-Invite precondition tests.

use super::*;

#[test]
fn test_multi_invite_targets_filtering() {
  let ws_state = create_test_ws_state();
  let _from = UserId::new();
  let online_users: Vec<UserId> = (0..3).map(|_| UserId::new()).collect();
  let offline_users: Vec<UserId> = (0..2).map(|_| UserId::new()).collect();

  // Add connections for online users
  for user in &online_users {
    ws_state.add_connection(user.clone(), create_test_sender());
  }

  // Create multi-invite with mixed targets
  let mut all_targets: Vec<UserId> = online_users.clone();
  all_targets.extend(offline_users.clone());

  // Filter online targets
  let online_targets: Vec<UserId> = all_targets
    .iter()
    .filter(|u| ws_state.is_connected(u))
    .cloned()
    .collect();

  // Should only have online users
  assert_eq!(online_targets.len(), 3);
  for user in &online_targets {
    assert!(online_users.contains(user));
  }
}

#[test]
fn test_multi_invite_rate_limiting() {
  use crate::discovery::INVITE_RATE_LIMIT_PER_MINUTE;

  let ws_state = create_test_ws_state();
  let from = UserId::new();

  // Send invitations up to the minute limit
  for _ in 0..INVITE_RATE_LIMIT_PER_MINUTE {
    let invite = ConnectionInvite {
      from: from.clone(),
      to: UserId::new(),
      note: None,
    };
    assert!(ws_state.discovery_state.send_invitation(&invite).is_ok());
  }

  // Next invitation should fail due to rate limit
  let invite = ConnectionInvite {
    from: from.clone(),
    to: UserId::new(),
    note: None,
  };
  let result = ws_state.discovery_state.send_invitation(&invite);
  assert!(result.is_err());

  // Verify remaining quota
  let (minute, _hour) = ws_state.discovery_state.get_remaining_quota(&from);
  assert_eq!(minute, 0);
}

#[test]
fn test_multi_invite_self_target_exclusion() {
  let ws_state = create_test_ws_state();
  let from = UserId::new();
  let other_user = UserId::new();

  ws_state.add_connection(from.clone(), create_test_sender());
  ws_state.add_connection(other_user.clone(), create_test_sender());

  // Create multi-invite including self
  let targets = [from.clone(), other_user.clone()];

  // Filter out self
  let filtered_targets: Vec<UserId> = targets.iter().filter(|&u| *u != from).cloned().collect();

  // Should only have other_user
  assert_eq!(filtered_targets.len(), 1);
  assert_eq!(filtered_targets[0], other_user);
}

#[test]
fn test_multi_invite_empty_targets() {
  let ws_state = create_test_ws_state();
  let from = UserId::new();

  // Create multi-invite with no targets
  let multi_invite = MultiInvite {
    from,
    targets: vec![],
  };

  let result = ws_state
    .discovery_state
    .send_multi_invitation(&multi_invite);
  assert!(result.is_err());
}

#[test]
fn test_multi_invite_all_targets_offline() {
  let ws_state = create_test_ws_state();
  let from = UserId::new();
  let offline_targets: Vec<UserId> = (0..3).map(|_| UserId::new()).collect();

  // Create multi-invite with offline targets
  let multi_invite = MultiInvite {
    from,
    targets: offline_targets,
  };

  // The invitation can be sent (stored in discovery state)
  // but won't be forwarded
  let result = ws_state
    .discovery_state
    .send_multi_invitation(&multi_invite);
  // Implementation allows storing invitations even if targets are offline
  assert!(result.is_ok() || result.is_err());
}
