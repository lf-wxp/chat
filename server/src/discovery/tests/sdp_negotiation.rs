use super::*;
use message::UserId;

#[test]
fn test_start_sdp_negotiation() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();

  let result = state.start_sdp_negotiation(&user1, &user2);
  assert!(result);
  assert!(state.is_sdp_negotiation_in_progress(&user1, &user2));
}

#[test]
fn test_sdp_negotiation_offer_sent() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();

  state.start_sdp_negotiation(&user1, &user2);
  state.mark_offer_sent(&user1, &user2);

  // Should be in progress now (offer sent, answer not received)
  assert!(state.is_sdp_negotiation_in_progress(&user1, &user2));
}

#[test]
fn test_sdp_negotiation_complete() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();

  state.start_sdp_negotiation(&user1, &user2);
  state.mark_offer_sent(&user1, &user2);
  state.mark_answer_received(&user1, &user2);

  // Should not be in progress after answer received
  assert!(!state.is_sdp_negotiation_in_progress(&user1, &user2));

  // Complete the negotiation
  state.complete_sdp_negotiation(&user1, &user2);

  // Check negotiation is removed
  let pending = state.get_pending_sdp_negotiations(&user1);
  assert!(pending.is_empty());
}

#[test]
fn test_duplicate_sdp_negotiation() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();

  // Start first negotiation
  let result1 = state.start_sdp_negotiation(&user1, &user2);
  assert!(result1);

  state.mark_offer_sent(&user1, &user2);

  // Try to start another negotiation while one is in progress
  let result2 = state.start_sdp_negotiation(&user1, &user2);
  assert!(!result2); // Should fail as one is already in progress
}

#[test]
fn test_clear_sdp_negotiations_for_user() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();
  let user3 = UserId::new();

  // user1 starts negotiations with user2 and user3
  state.start_sdp_negotiation(&user1, &user2);
  state.start_sdp_negotiation(&user1, &user3);
  state.mark_offer_sent(&user1, &user2);
  state.mark_offer_sent(&user1, &user3);

  // Clear all negotiations for user1
  state.clear_sdp_negotiations_for_user(&user1);

  // Check no pending negotiations
  let pending = state.get_pending_sdp_negotiations(&user1);
  assert!(pending.is_empty());
}

#[test]
fn test_sdp_negotiation_state_initial() {
  let user1 = UserId::new();
  let user2 = UserId::new();
  let state = SdpNegotiationState::new(user1.clone(), user2.clone());

  assert_eq!(state.initiator, user1);
  assert_eq!(state.target, user2);
  assert!(!state.offer_sent);
  assert!(!state.answer_received);
  // Initially in progress (negotiation started, waiting for offer to be sent)
  assert!(state.is_in_progress());
  assert!(!state.is_complete());
  assert!(!state.is_timed_out());
}

#[test]
fn test_sdp_negotiation_state_in_progress() {
  let user1 = UserId::new();
  let user2 = UserId::new();
  let mut state = SdpNegotiationState::new(user1, user2);
  state.offer_sent = true;

  assert!(state.is_in_progress());
  assert!(!state.is_complete());
}

#[test]
fn test_sdp_negotiation_state_complete() {
  let user1 = UserId::new();
  let user2 = UserId::new();
  let mut state = SdpNegotiationState::new(user1, user2);
  state.offer_sent = true;
  state.answer_received = true;

  assert!(!state.is_in_progress());
  assert!(state.is_complete());
}
