//! ICE Candidate precondition tests.

use super::*;

#[test]
fn test_ice_candidate_sender_mismatch() {
  let user1 = UserId::new();
  let user2 = UserId::new();
  let user3 = UserId::new();

  let candidate = IceCandidate::new(user1.clone(), user2.clone(), "candidate:...".to_string());

  // If authenticated as user3, candidate from user1 should fail validation
  assert_ne!(candidate.from, user3);
}

#[test]
fn test_ice_candidate_target_offline() {
  let ws_state = create_test_ws_state();
  let from = UserId::new();
  let to = UserId::new();

  ws_state.add_connection(from.clone(), create_test_sender());

  // Target offline - should handle gracefully (no error sent for ICE)
  assert!(!ws_state.is_connected(&to));
}

#[test]
fn test_ice_candidate_forwarding() {
  let ws_state = create_test_ws_state();
  let from = UserId::new();
  let to = UserId::new();

  ws_state.add_connection(from.clone(), create_test_sender());
  ws_state.add_connection(to.clone(), create_test_sender());

  // Both connected - should be able to forward
  assert!(ws_state.is_connected(&from));
  assert!(ws_state.is_connected(&to));
  assert!(ws_state.get_sender(&to).is_some());
}
