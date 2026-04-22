use super::*;

// ── PeerConnectionState tests ──

#[test]
fn test_peer_connection_state_from_known_strings() {
  assert_eq!(
    PeerConnectionState::from("connecting"),
    PeerConnectionState::Connecting
  );
  assert_eq!(
    PeerConnectionState::from("connected"),
    PeerConnectionState::Connected
  );
  assert_eq!(
    PeerConnectionState::from("disconnected"),
    PeerConnectionState::Disconnected
  );
  assert_eq!(
    PeerConnectionState::from("failed"),
    PeerConnectionState::Failed
  );
  assert_eq!(
    PeerConnectionState::from("closed"),
    PeerConnectionState::Closed
  );
}

#[test]
fn test_peer_connection_state_from_unknown_defaults_to_closed() {
  assert_eq!(
    PeerConnectionState::from("invalid"),
    PeerConnectionState::Closed
  );
  assert_eq!(PeerConnectionState::from(""), PeerConnectionState::Closed);
  assert_eq!(
    PeerConnectionState::from("CONNECTED"),
    PeerConnectionState::Closed
  );
}

#[test]
fn test_peer_connection_state_eq_and_copy() {
  let state = PeerConnectionState::Connected;
  let copy = state;
  assert_eq!(state, copy);
}

// ── DataChannelState tests ──

#[test]
fn test_data_channel_state_from_known_strings() {
  assert_eq!(
    DataChannelState::from("connecting"),
    DataChannelState::Connecting
  );
  assert_eq!(DataChannelState::from("open"), DataChannelState::Open);
  assert_eq!(DataChannelState::from("closing"), DataChannelState::Closing);
  assert_eq!(DataChannelState::from("closed"), DataChannelState::Closed);
}

#[test]
fn test_data_channel_state_from_unknown_defaults_to_closed() {
  assert_eq!(DataChannelState::from("unknown"), DataChannelState::Closed);
  assert_eq!(DataChannelState::from(""), DataChannelState::Closed);
  assert_eq!(DataChannelState::from("OPEN"), DataChannelState::Closed);
}

// ── PeerEncryptionKeys tests ──

#[test]
fn test_peer_encryption_keys_clone() {
  let keys = PeerEncryptionKeys {
    aes_key: vec![0u8; 32],
    key_id: 1,
  };
  let cloned = keys.clone();
  assert_eq!(cloned.aes_key.len(), 32);
  assert_eq!(cloned.key_id, 1);
}

// ── PeerState tests ──

#[test]
fn test_peer_state_new_defaults() {
  let user_id = UserId::new();
  let state = PeerState::new(user_id.clone(), true);
  assert_eq!(state.user_id, user_id);
  assert_eq!(state.connection_state, PeerConnectionState::Connecting);
  assert!(state.data_channel_state.is_none());
  assert!(state.encryption_keys.is_none());
  assert!(state.is_initiator);
}

#[test]
fn test_peer_state_new_receiver() {
  let user_id = UserId::new();
  let state = PeerState::new(user_id, false);
  assert!(!state.is_initiator);
}

#[test]
fn test_peer_state_is_ready_requires_both_connected_and_open() {
  let user_id = UserId::new();
  let mut state = PeerState::new(user_id, true);

  // Initially not ready (connecting, no data channel)
  assert!(!state.is_ready());

  // Connected but no data channel -> not ready
  state.connection_state = PeerConnectionState::Connected;
  assert!(!state.is_ready());

  // Connected + data channel connecting -> not ready
  state.data_channel_state = Some(DataChannelState::Connecting);
  assert!(!state.is_ready());

  // Connected + data channel open -> ready!
  state.data_channel_state = Some(DataChannelState::Open);
  assert!(state.is_ready());
}

#[test]
fn test_peer_state_is_ready_false_when_disconnected() {
  let user_id = UserId::new();
  let mut state = PeerState::new(user_id, true);
  state.connection_state = PeerConnectionState::Disconnected;
  state.data_channel_state = Some(DataChannelState::Open);
  assert!(!state.is_ready());
}

#[test]
fn test_peer_state_is_ready_false_when_failed() {
  let user_id = UserId::new();
  let mut state = PeerState::new(user_id, true);
  state.connection_state = PeerConnectionState::Failed;
  state.data_channel_state = Some(DataChannelState::Open);
  assert!(!state.is_ready());
}

#[test]
fn test_peer_state_is_ready_false_when_channel_closing() {
  let user_id = UserId::new();
  let mut state = PeerState::new(user_id, true);
  state.connection_state = PeerConnectionState::Connected;
  state.data_channel_state = Some(DataChannelState::Closing);
  assert!(!state.is_ready());
}

// ── WebRtcState tests ──

#[test]
fn test_webrtc_state_new_is_empty() {
  let state = WebRtcState::new();
  assert!(state.peers.is_empty());
  assert_eq!(state.connected_count(), 0);
}

#[test]
fn test_webrtc_state_default_is_empty() {
  let state = WebRtcState::default();
  assert!(state.peers.is_empty());
}

#[test]
fn test_webrtc_state_add_peer() {
  let mut state = WebRtcState::new();
  let user_id = UserId::new();
  state.add_peer(user_id.clone(), true);
  assert_eq!(state.peers.len(), 1);
  assert!(state.get_peer(&user_id).is_some());
  assert!(state.get_peer(&user_id).unwrap().is_initiator);
}

#[test]
fn test_webrtc_state_add_multiple_peers() {
  let mut state = WebRtcState::new();
  let user1 = UserId::new();
  let user2 = UserId::new();
  let user3 = UserId::new();
  state.add_peer(user1, true);
  state.add_peer(user2, false);
  state.add_peer(user3, true);
  assert_eq!(state.peers.len(), 3);
}

#[test]
fn test_webrtc_state_remove_peer() {
  let mut state = WebRtcState::new();
  let user_id = UserId::new();
  state.add_peer(user_id.clone(), true);
  assert_eq!(state.peers.len(), 1);
  state.remove_peer(&user_id);
  assert!(state.peers.is_empty());
  assert!(state.get_peer(&user_id).is_none());
}

#[test]
fn test_webrtc_state_remove_nonexistent_peer() {
  let mut state = WebRtcState::new();
  let user_id = UserId::new();
  // Should not panic
  state.remove_peer(&user_id);
  assert!(state.peers.is_empty());
}

#[test]
fn test_webrtc_state_get_peer_mut() {
  let mut state = WebRtcState::new();
  let user_id = UserId::new();
  state.add_peer(user_id.clone(), true);
  let peer = state.get_peer_mut(&user_id).unwrap();
  peer.connection_state = PeerConnectionState::Connected;
  assert_eq!(
    state.get_peer(&user_id).unwrap().connection_state,
    PeerConnectionState::Connected
  );
}

#[test]
fn test_webrtc_state_update_connection_state() {
  let mut state = WebRtcState::new();
  let user_id = UserId::new();
  state.add_peer(user_id.clone(), true);

  state.update_connection_state(&user_id, PeerConnectionState::Connected);
  assert_eq!(
    state.get_peer(&user_id).unwrap().connection_state,
    PeerConnectionState::Connected
  );

  state.update_connection_state(&user_id, PeerConnectionState::Failed);
  assert_eq!(
    state.get_peer(&user_id).unwrap().connection_state,
    PeerConnectionState::Failed
  );
}

#[test]
fn test_webrtc_state_update_connection_state_nonexistent() {
  let mut state = WebRtcState::new();
  let user_id = UserId::new();
  // Should not panic when updating a non-existent peer
  state.update_connection_state(&user_id, PeerConnectionState::Connected);
}

#[test]
fn test_webrtc_state_update_data_channel_state() {
  let mut state = WebRtcState::new();
  let user_id = UserId::new();
  state.add_peer(user_id.clone(), true);

  state.update_data_channel_state(&user_id, DataChannelState::Open);
  assert_eq!(
    state.get_peer(&user_id).unwrap().data_channel_state,
    Some(DataChannelState::Open)
  );
}

#[test]
fn test_webrtc_state_set_encryption_keys() {
  let mut state = WebRtcState::new();
  let user_id = UserId::new();
  state.add_peer(user_id.clone(), true);

  let keys = PeerEncryptionKeys {
    aes_key: vec![42u8; 32],
    key_id: 7,
  };
  state.set_encryption_keys(&user_id, keys);

  let peer = state.get_peer(&user_id).unwrap();
  assert!(peer.encryption_keys.is_some());
  let stored_keys = peer.encryption_keys.as_ref().unwrap();
  assert_eq!(stored_keys.key_id, 7);
  assert_eq!(stored_keys.aes_key.len(), 32);
}

#[test]
fn test_webrtc_state_connected_count_none_ready() {
  let mut state = WebRtcState::new();
  state.add_peer(UserId::new(), true);
  state.add_peer(UserId::new(), false);
  // All are in Connecting state, none ready
  assert_eq!(state.connected_count(), 0);
}

#[test]
fn test_webrtc_state_connected_count_some_ready() {
  let mut state = WebRtcState::new();
  let user1 = UserId::new();
  let user2 = UserId::new();
  let user3 = UserId::new();
  state.add_peer(user1.clone(), true);
  state.add_peer(user2.clone(), false);
  state.add_peer(user3.clone(), true);

  // Make user1 fully ready
  state.update_connection_state(&user1, PeerConnectionState::Connected);
  state.update_data_channel_state(&user1, DataChannelState::Open);

  // user2 connected but no data channel
  state.update_connection_state(&user2, PeerConnectionState::Connected);

  // user3 still connecting
  assert_eq!(state.connected_count(), 1);
}

#[test]
fn test_webrtc_state_connected_count_all_ready() {
  let mut state = WebRtcState::new();
  let user1 = UserId::new();
  let user2 = UserId::new();
  state.add_peer(user1.clone(), true);
  state.add_peer(user2.clone(), false);

  for uid in [&user1, &user2] {
    state.update_connection_state(uid, PeerConnectionState::Connected);
    state.update_data_channel_state(uid, DataChannelState::Open);
  }

  assert_eq!(state.connected_count(), 2);
}

#[test]
fn test_webrtc_state_add_peer_replaces_existing() {
  let mut state = WebRtcState::new();
  let user_id = UserId::new();
  state.add_peer(user_id.clone(), true);
  state.update_connection_state(&user_id, PeerConnectionState::Connected);

  // Re-adding with different initiator flag should replace
  state.add_peer(user_id.clone(), false);
  let peer = state.get_peer(&user_id).unwrap();
  assert!(!peer.is_initiator);
  assert_eq!(peer.connection_state, PeerConnectionState::Connecting);
}
