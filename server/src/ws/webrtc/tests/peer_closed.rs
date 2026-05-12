//! Peer Closed precondition tests.

use super::*;

#[test]
fn test_peer_closed_removes_active_peer() {
  let ws_state = create_test_ws_state();
  let from = UserId::new();
  let to = UserId::new();

  // Add active peer
  ws_state.discovery_state.add_active_peer(&from, &to);
  assert!(ws_state.discovery_state.are_peers(&from, &to));

  // Close peer connection
  ws_state.discovery_state.remove_active_peer(&from, &to);

  // Should no longer be peers
  assert!(!ws_state.discovery_state.are_peers(&from, &to));
}

#[test]
fn test_peer_closed_sender_validation() {
  let user1 = UserId::new();
  let user2 = UserId::new();
  let user3 = UserId::new();

  let peer_closed = PeerClosed {
    from: user1.clone(),
    to: user2.clone(),
  };

  // Validate sender
  assert_eq!(peer_closed.from, user1);
  assert_ne!(peer_closed.from, user3);
}

#[test]
fn test_peer_closed_clears_negotiation() {
  let ws_state = create_test_ws_state();
  let from = UserId::new();
  let to = UserId::new();

  // Start negotiation
  ws_state.discovery_state.start_sdp_negotiation(&from, &to);

  // Complete via peer closed
  ws_state
    .discovery_state
    .complete_sdp_negotiation(&from, &to);

  // Negotiation should be cleared
  assert!(
    !ws_state
      .discovery_state
      .is_sdp_negotiation_in_progress(&from, &to)
  );
}
