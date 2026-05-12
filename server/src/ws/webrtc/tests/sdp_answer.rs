//! SDP Answer precondition tests.

use super::*;

#[test]
fn test_sdp_answer_completes_negotiation() {
  let ws_state = create_test_ws_state();
  let from = UserId::new();
  let to = UserId::new();

  ws_state.add_connection(from.clone(), create_test_sender());
  ws_state.add_connection(to.clone(), create_test_sender());

  // Start negotiation (offer direction: from -> to)
  ws_state.discovery_state.start_sdp_negotiation(&from, &to);

  // Mark answer received
  ws_state.discovery_state.mark_answer_received(&from, &to);

  // Negotiation should be complete
  assert!(
    !ws_state
      .discovery_state
      .is_sdp_negotiation_in_progress(&from, &to)
  );
}

#[test]
fn test_sdp_answer_without_offer() {
  let ws_state = create_test_ws_state();
  let from = UserId::new();
  let to = UserId::new();

  // Mark answer received without starting negotiation
  ws_state.discovery_state.mark_answer_received(&from, &to);

  // Should not be in progress
  assert!(
    !ws_state
      .discovery_state
      .is_sdp_negotiation_in_progress(&from, &to)
  );
}
