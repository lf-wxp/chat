//! Active peers management tests.

use super::*;

#[test]
fn test_add_active_peer_bidirectional() {
  let state = DiscoveryState::new();
  let user_a = UserId::new();
  let user_b = UserId::new();

  state.add_active_peer(&user_a, &user_b);

  assert!(state.are_peers(&user_a, &user_b));
  assert!(state.are_peers(&user_b, &user_a));
}

#[test]
fn test_remove_active_peer() {
  let state = DiscoveryState::new();
  let user_a = UserId::new();
  let user_b = UserId::new();

  state.add_active_peer(&user_a, &user_b);
  state.remove_active_peer(&user_a, &user_b);

  assert!(!state.are_peers(&user_a, &user_b));
  assert!(!state.are_peers(&user_b, &user_a));
}

#[test]
fn test_get_active_peers_multiple() {
  let state = DiscoveryState::new();
  let user_a = UserId::new();
  let user_b = UserId::new();
  let user_c = UserId::new();

  state.add_active_peer(&user_a, &user_b);
  state.add_active_peer(&user_a, &user_c);

  let peers = state.get_active_peers(&user_a);
  assert_eq!(peers.len(), 2);
  assert!(peers.contains(&user_b));
  assert!(peers.contains(&user_c));
}

#[test]
fn test_are_peers_false_when_no_relationship() {
  let state = DiscoveryState::new();
  let user_a = UserId::new();
  let user_b = UserId::new();

  assert!(!state.are_peers(&user_a, &user_b));
}

#[test]
fn test_clear_active_peers() {
  let state = DiscoveryState::new();
  let user_a = UserId::new();
  let user_b = UserId::new();
  let user_c = UserId::new();

  state.add_active_peer(&user_a, &user_b);
  state.add_active_peer(&user_a, &user_c);

  state.clear_active_peers(&user_a);

  assert!(state.get_active_peers(&user_a).is_empty());
  assert!(!state.are_peers(&user_b, &user_a));
  assert!(!state.are_peers(&user_c, &user_a));
}
