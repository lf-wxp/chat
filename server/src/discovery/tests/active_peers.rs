use super::*;
use message::UserId;

#[test]
fn test_add_active_peer() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();

  state.add_active_peer(&user1, &user2);

  // Check bidirectional relationship
  assert!(state.are_peers(&user1, &user2));
  assert!(state.are_peers(&user2, &user1));

  // Check get_active_peers
  let peers1 = state.get_active_peers(&user1);
  assert_eq!(peers1.len(), 1);
  assert!(peers1.contains(&user2));

  let peers2 = state.get_active_peers(&user2);
  assert_eq!(peers2.len(), 1);
  assert!(peers2.contains(&user1));
}

#[test]
fn test_remove_active_peer() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();

  state.add_active_peer(&user1, &user2);
  assert!(state.are_peers(&user1, &user2));

  state.remove_active_peer(&user1, &user2);

  // Check relationship is removed bidirectionally
  assert!(!state.are_peers(&user1, &user2));
  assert!(!state.are_peers(&user2, &user1));

  // Check get_active_peers returns empty
  assert!(state.get_active_peers(&user1).is_empty());
  assert!(state.get_active_peers(&user2).is_empty());
}

#[test]
fn test_multiple_active_peers() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();
  let user3 = UserId::new();

  // User1 connects with user2 and user3
  state.add_active_peer(&user1, &user2);
  state.add_active_peer(&user1, &user3);

  // Check user1 has two peers
  let peers1 = state.get_active_peers(&user1);
  assert_eq!(peers1.len(), 2);
  assert!(peers1.contains(&user2));
  assert!(peers1.contains(&user3));

  // Check user2 and user3 have user1 as peer
  assert!(state.are_peers(&user2, &user1));
  assert!(state.are_peers(&user3, &user1));

  // Remove one peer
  state.remove_active_peer(&user1, &user2);
  let peers1 = state.get_active_peers(&user1);
  assert_eq!(peers1.len(), 1);
  assert!(peers1.contains(&user3));

  // User2 should no longer have user1 as peer
  assert!(!state.are_peers(&user2, &user1));
}

#[test]
fn test_clear_active_peers() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();
  let user3 = UserId::new();

  // user1 connects with user2 and user3
  state.add_active_peer(&user1, &user2);
  state.add_active_peer(&user1, &user3);

  // Clear all peers for user1
  state.clear_active_peers(&user1);

  // Check user1 has no peers
  assert!(state.get_active_peers(&user1).is_empty());

  // Check user2 and user3 no longer have user1 as peer
  assert!(!state.are_peers(&user2, &user1));
  assert!(!state.are_peers(&user3, &user1));
}

#[test]
fn test_remove_nonexistent_peer() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();

  // Should not panic
  state.remove_active_peer(&user1, &user2);

  assert!(!state.are_peers(&user1, &user2));
}

#[test]
fn test_add_same_peer_twice() {
  let state = create_test_state();
  let user1 = UserId::new();
  let user2 = UserId::new();

  state.add_active_peer(&user1, &user2);
  state.add_active_peer(&user1, &user2); // Add again

  // Should still have only one peer
  let peers = state.get_active_peers(&user1);
  assert_eq!(peers.len(), 1);
}

#[test]
fn test_multiple_peer_connections() {
  let state = create_test_state();
  let hub_user = UserId::new();
  let peers: Vec<UserId> = (0..10).map(|_| UserId::new()).collect();

  // Hub user connects to all peers
  for peer in &peers {
    state.add_active_peer(&hub_user, peer);
  }

  // Verify hub has all peers
  let hub_peers = state.get_active_peers(&hub_user);
  assert_eq!(hub_peers.len(), 10);

  // Each peer should have hub as their peer
  for peer in &peers {
    assert!(state.are_peers(peer, &hub_user));
  }

  // Remove half the peers
  for peer in &peers[0..5] {
    state.remove_active_peer(&hub_user, peer);
  }

  // Verify hub now has 5 peers
  let hub_peers = state.get_active_peers(&hub_user);
  assert_eq!(hub_peers.len(), 5);

  // Removed peers should no longer have hub as peer
  for peer in &peers[0..5] {
    assert!(!state.are_peers(peer, &hub_user));
  }

  // Remaining peers should still have hub
  for peer in &peers[5..10] {
    assert!(state.are_peers(peer, &hub_user));
  }
}
