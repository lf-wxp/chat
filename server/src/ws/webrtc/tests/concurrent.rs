//! Concurrent WebRTC operations tests.

use super::*;
use std::sync::Arc;

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
