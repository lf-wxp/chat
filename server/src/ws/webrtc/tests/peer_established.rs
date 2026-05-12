//! Peer Established precondition tests.

use super::*;

#[test]
fn test_peer_established_adds_active_peer() {
  let ws_state = create_test_ws_state();
  let from = UserId::new();
  let to = UserId::new();

  ws_state.add_connection(from.clone(), create_test_sender());
  ws_state.add_connection(to.clone(), create_test_sender());

  // Add active peer relationship
  ws_state.discovery_state.add_active_peer(&from, &to);

  // Verify peer relationship
  assert!(ws_state.discovery_state.are_peers(&from, &to));
  assert!(ws_state.discovery_state.are_peers(&to, &from));
}

#[test]
fn test_peer_established_clears_negotiation() {
  let ws_state = create_test_ws_state();
  let from = UserId::new();
  let to = UserId::new();

  // Start negotiation
  ws_state.discovery_state.start_sdp_negotiation(&from, &to);

  // Complete negotiation
  ws_state
    .discovery_state
    .complete_sdp_negotiation(&from, &to);

  // Should not be in progress
  assert!(
    !ws_state
      .discovery_state
      .is_sdp_negotiation_in_progress(&from, &to)
  );
}
