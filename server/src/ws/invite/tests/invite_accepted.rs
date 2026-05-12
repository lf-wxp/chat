//! Invite Accepted flow tests.

use super::*;

#[test]
fn test_invite_accepted_flow() {
  let ws_state = create_test_ws_state();
  let user1 = UserId::new();
  let user2 = UserId::new();

  // Add connections
  ws_state.add_connection(user1.clone(), create_test_sender());
  ws_state.add_connection(user2.clone(), create_test_sender());

  // Create invitations
  let invite1 = ConnectionInvite {
    from: user1.clone(),
    to: user2.clone(),
    note: None,
  };
  ws_state.discovery_state.send_invitation(&invite1).unwrap();

  let invite2 = ConnectionInvite {
    from: user2.clone(),
    to: user1.clone(),
    note: None,
  };
  ws_state.discovery_state.send_invitation(&invite2).unwrap();

  // Merge bidirectional invitations
  let merged = ws_state
    .discovery_state
    .merge_bidirectional_invitations(&user1, &user2);

  // Should have merged
  assert!(merged.is_some());

  // Both should be active peers now
  ws_state.discovery_state.add_active_peer(&user1, &user2);
  let peers1 = ws_state.discovery_state.get_active_peers(&user1);
  let peers2 = ws_state.discovery_state.get_active_peers(&user2);

  assert!(peers1.contains(&user2));
  assert!(peers2.contains(&user1));
}
