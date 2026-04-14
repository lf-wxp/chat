//! Peer discovery and SDP negotiation tests.

use super::*;

#[test]
fn test_active_peers_tracking() {
  let ws_state = Arc::new(create_test_ws_state());
  let user1 = UserId::new();
  let user2 = UserId::new();
  let user3 = UserId::new();

  // Initially no peers
  assert!(ws_state.discovery_state.get_active_peers(&user1).is_empty());

  // Add peer relationship
  ws_state.discovery_state.add_active_peer(&user1, &user2);

  // Check bidirectional relationship
  assert!(ws_state.discovery_state.are_peers(&user1, &user2));
  assert!(ws_state.discovery_state.are_peers(&user2, &user1));

  // Check peers list
  let peers1 = ws_state.discovery_state.get_active_peers(&user1);
  assert_eq!(peers1.len(), 1);
  assert!(peers1.contains(&user2));

  // Add another peer
  ws_state.discovery_state.add_active_peer(&user1, &user3);
  let peers1 = ws_state.discovery_state.get_active_peers(&user1);
  assert_eq!(peers1.len(), 2);

  // Remove peer
  ws_state.discovery_state.remove_active_peer(&user1, &user2);
  let peers1 = ws_state.discovery_state.get_active_peers(&user1);
  assert_eq!(peers1.len(), 1);
  assert!(!peers1.contains(&user2));

  // Clear all peers
  ws_state.discovery_state.clear_active_peers(&user1);
  assert!(ws_state.discovery_state.get_active_peers(&user1).is_empty());
  assert!(!ws_state.discovery_state.are_peers(&user1, &user3));
}

#[test]
fn test_sdp_negotiation_tracking() {
  let ws_state = Arc::new(create_test_ws_state());
  let user1 = UserId::new();
  let user2 = UserId::new();

  // Start SDP negotiation
  let started = ws_state
    .discovery_state
    .start_sdp_negotiation(&user1, &user2);
  assert!(started);

  // Cannot start another negotiation while one is in progress
  let started_again = ws_state
    .discovery_state
    .start_sdp_negotiation(&user1, &user2);
  assert!(!started_again);

  // Mark offer sent
  ws_state.discovery_state.mark_offer_sent(&user1, &user2);

  // Should be in progress now
  assert!(
    ws_state
      .discovery_state
      .is_sdp_negotiation_in_progress(&user1, &user2)
  );

  // Mark answer received
  ws_state
    .discovery_state
    .mark_answer_received(&user1, &user2);

  // Should not be in progress after answer received
  assert!(
    !ws_state
      .discovery_state
      .is_sdp_negotiation_in_progress(&user1, &user2)
  );

  // Complete negotiation
  ws_state
    .discovery_state
    .complete_sdp_negotiation(&user1, &user2);

  // Can start new negotiation now
  let started_new = ws_state
    .discovery_state
    .start_sdp_negotiation(&user1, &user2);
  assert!(started_new);
}
