//! Invite Declined flow tests.

use super::*;

#[test]
fn test_invite_rejection_flow() {
  let ws_state = create_test_ws_state();
  let inviter = UserId::new();
  let invitee = UserId::new();

  // Add connections
  ws_state.add_connection(inviter.clone(), create_test_sender());
  ws_state.add_connection(invitee.clone(), create_test_sender());

  // Send invitation
  let invite = ConnectionInvite {
    from: inviter.clone(),
    to: invitee.clone(),
    note: Some("Hello!".to_string()),
  };
  ws_state.discovery_state.send_invitation(&invite).unwrap();

  // Reject invitation
  let declined = ws_state
    .discovery_state
    .decline_invitation(&inviter, &invitee);
  assert!(declined.is_some());

  // Verify no active peer relationship
  let peers = ws_state.discovery_state.get_active_peers(&inviter);
  assert!(!peers.contains(&invitee));
}
