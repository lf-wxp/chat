//! SDP negotiation management tests.

use super::*;

#[test]
fn test_start_sdp_negotiation() {
  let state = DiscoveryState::new();
  let from = UserId::new();
  let to = UserId::new();

  let started = state.start_sdp_negotiation(&from, &to);
  assert!(started);
  assert!(state.is_sdp_negotiation_in_progress(&from, &to));
}

#[test]
fn test_start_sdp_negotiation_already_in_progress() {
  let state = DiscoveryState::new();
  let from = UserId::new();
  let to = UserId::new();

  state.start_sdp_negotiation(&from, &to);
  let second = state.start_sdp_negotiation(&from, &to);
  assert!(!second);
}

#[test]
fn test_mark_offer_sent_and_answer_received() {
  let state = DiscoveryState::new();
  let from = UserId::new();
  let to = UserId::new();

  state.start_sdp_negotiation(&from, &to);
  state.mark_offer_sent(&from, &to);
  state.mark_answer_received(&to, &from);

  // Negotiation should still be in progress until completed
  assert!(state.is_sdp_negotiation_in_progress(&from, &to));
}

#[test]
fn test_complete_sdp_negotiation() {
  let state = DiscoveryState::new();
  let from = UserId::new();
  let to = UserId::new();

  state.start_sdp_negotiation(&from, &to);
  state.complete_sdp_negotiation(&from, &to);

  assert!(!state.is_sdp_negotiation_in_progress(&from, &to));
}

#[test]
fn test_get_pending_sdp_negotiations() {
  let state = DiscoveryState::new();
  let user_a = UserId::new();
  let user_b = UserId::new();

  state.start_sdp_negotiation(&user_a, &user_b);

  let pending = state.get_pending_sdp_negotiations(&user_a);
  assert_eq!(pending.len(), 1);

  let pending_b = state.get_pending_sdp_negotiations(&user_b);
  assert_eq!(pending_b.len(), 1);
}

#[test]
fn test_clear_sdp_negotiations_for_user() {
  let state = DiscoveryState::new();
  let user_a = UserId::new();
  let user_b = UserId::new();
  let user_c = UserId::new();

  state.start_sdp_negotiation(&user_a, &user_b);
  state.start_sdp_negotiation(&user_a, &user_c);

  state.clear_sdp_negotiations_for_user(&user_a);

  assert!(state.get_pending_sdp_negotiations(&user_a).is_empty());
  assert!(state.get_pending_sdp_negotiations(&user_b).is_empty());
  assert!(state.get_pending_sdp_negotiations(&user_c).is_empty());
}
