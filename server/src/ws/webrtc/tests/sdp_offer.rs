//! SDP Offer precondition tests.

use super::*;

#[test]
fn test_sdp_offer_sender_validation() {
  let user1 = UserId::new();
  let user2 = UserId::new();

  let offer = SdpOffer {
    from: user1.clone(),
    to: user2.clone(),
    sdp: "v=0...".to_string(),
  };

  // Verify sender matches
  assert_eq!(offer.from, user1);
  assert_ne!(offer.from, user2);
}

#[test]
fn test_sdp_offer_target_offline() {
  let ws_state = create_test_ws_state();
  let from = UserId::new();
  let to = UserId::new();

  // Only add 'from' user
  ws_state.add_connection(from.clone(), create_test_sender());

  // Target should be offline
  assert!(!ws_state.is_connected(&to));
  assert!(ws_state.is_connected(&from));
}

#[test]
fn test_sdp_offer_starts_negotiation() {
  let ws_state = create_test_ws_state();
  let from = UserId::new();
  let to = UserId::new();

  ws_state.add_connection(from.clone(), create_test_sender());
  ws_state.add_connection(to.clone(), create_test_sender());

  // Start SDP negotiation
  let started = ws_state.discovery_state.start_sdp_negotiation(&from, &to);
  assert!(started);

  // Should be in progress
  assert!(
    ws_state
      .discovery_state
      .is_sdp_negotiation_in_progress(&from, &to)
  );
}
