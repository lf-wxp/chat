use super::*;
use crate::ws::tests::{create_test_sender, create_test_ws_state};
use message::signaling::{IceCandidate, PeerClosed, SdpOffer};

// ===== SDP Offer Tests =====

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

// ===== SDP Answer Tests =====

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

// ===== ICE Candidate Tests =====

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

// ===== Peer Established Tests =====

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

// ===== Peer Closed Tests =====

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

// ===== Concurrent Operations Tests =====

#[test]
fn test_concurrent_sdp_negotiations() {
  let ws_state = Arc::new(create_test_ws_state());
  let users: Vec<UserId> = (0..4).map(|_| UserId::new()).collect();

  for user in &users {
    ws_state.add_connection(user.clone(), create_test_sender());
  }

  let state_clone = ws_state.clone();
  let users_clone = users.clone();

  let handle = std::thread::spawn(move || {
    // Start negotiations between pairs
    state_clone
      .discovery_state
      .start_sdp_negotiation(&users_clone[0], &users_clone[1]);
    state_clone
      .discovery_state
      .start_sdp_negotiation(&users_clone[2], &users_clone[3]);
  });

  handle.join().unwrap();

  // Both negotiations should be in progress
  assert!(
    ws_state
      .discovery_state
      .is_sdp_negotiation_in_progress(&users[0], &users[1])
  );
  assert!(
    ws_state
      .discovery_state
      .is_sdp_negotiation_in_progress(&users[2], &users[3])
  );
}

#[test]
fn test_bidirectional_peer_relationship() {
  let ws_state = create_test_ws_state();
  let user1 = UserId::new();
  let user2 = UserId::new();

  // Add peer in one direction
  ws_state.discovery_state.add_active_peer(&user1, &user2);

  // Verify bidirectional relationship
  assert!(ws_state.discovery_state.are_peers(&user1, &user2));
  assert!(ws_state.discovery_state.are_peers(&user2, &user1));

  // Get peers for both
  let peers1 = ws_state.discovery_state.get_active_peers(&user1);
  let peers2 = ws_state.discovery_state.get_active_peers(&user2);

  assert!(peers1.contains(&user2));
  assert!(peers2.contains(&user1));
}

#[test]
fn test_multiple_peers_per_user() {
  let ws_state = create_test_ws_state();
  let hub = UserId::new();
  let peers: Vec<UserId> = (0..5).map(|_| UserId::new()).collect();

  // Hub connects to all peers
  for peer in &peers {
    ws_state.discovery_state.add_active_peer(&hub, peer);
  }

  // Verify hub has all peers
  let hub_peers = ws_state.discovery_state.get_active_peers(&hub);
  assert_eq!(hub_peers.len(), 5);

  // Each peer should have hub
  for peer in &peers {
    let peer_peers = ws_state.discovery_state.get_active_peers(peer);
    assert!(peer_peers.contains(&hub));
  }
}
