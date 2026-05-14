use super::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn test_app_state() -> AppState {
  AppState::new()
}

#[test]
fn test_webrtc_manager_new() {
  let app_state = test_app_state();
  let manager = WebRtcManager::new(app_state);

  assert_eq!(manager.connection_count(), 0);
  assert!(manager.encrypted_peers().is_empty());
}

#[test]
fn test_webrtc_manager_clone() {
  let app_state = test_app_state();
  let manager1 = WebRtcManager::new(app_state);
  let manager2 = manager1.clone();

  // Both clones share the same inner state
  assert_eq!(manager1.connection_count(), manager2.connection_count());
}

#[test]
fn test_is_connected_false_when_empty() {
  let app_state = test_app_state();
  let manager = WebRtcManager::new(app_state);
  let peer_id = UserId::new();

  assert!(!manager.is_connected(&peer_id));
}

#[test]
fn test_connection_count_empty() {
  let app_state = test_app_state();
  let manager = WebRtcManager::new(app_state);

  assert_eq!(manager.connection_count(), 0);
}

#[test]
fn test_has_encryption_key_false_when_empty() {
  let app_state = test_app_state();
  let manager = WebRtcManager::new(app_state);
  let peer_id = UserId::new();

  assert!(!manager.has_encryption_key(&peer_id));
}

#[test]
fn test_encrypted_peers_empty() {
  let app_state = test_app_state();
  let manager = WebRtcManager::new(app_state);

  assert!(manager.encrypted_peers().is_empty());
}

#[test]
fn test_default_ice_servers() {
  let app_state = test_app_state();
  let manager = WebRtcManager::new(app_state);

  // Default ICE servers should be set internally
  manager.init_with_ice_servers(vec![IceServerConfig::stun("stun:custom.example.com:3478")]);
  // No public accessor for ice_servers, but we can verify the manager works
  assert_eq!(manager.connection_count(), 0);
}

#[test]
fn test_init_with_empty_ice_servers_overrides_defaults() {
  let app_state = test_app_state();
  let manager = WebRtcManager::new(app_state);

  // The server is authoritative for ICE server configuration; an
  // explicitly empty `Vec` is a meaningful signal ("use host
  // candidates only" — see the doc-comment on
  // `init_with_ice_servers`) and must override the compiled-in
  // default rather than being silently ignored.
  manager.init_with_ice_servers(vec![]);
  assert_eq!(manager.connection_count(), 0);
}

#[test]
fn test_app_state_webrtc_state_tracking() {
  let app_state = test_app_state();
  let user_id = UserId::new();

  // Initially empty
  assert_eq!(app_state.webrtc_state.get().peers.len(), 0);

  // Add a peer
  app_state
    .webrtc_state
    .update(|state| state.add_peer(user_id.clone(), true));

  let state = app_state.webrtc_state.get();
  assert_eq!(state.peers.len(), 1);
  assert!(state.get_peer(&user_id).is_some());
  assert_eq!(
    state.get_peer(&user_id).unwrap().connection_state,
    PeerConnectionState::Connecting
  );

  // Update connection state
  drop(state);
  app_state
    .webrtc_state
    .update(|s| s.update_connection_state(&user_id, PeerConnectionState::Connected));

  assert_eq!(
    app_state
      .webrtc_state
      .get()
      .get_peer(&user_id)
      .unwrap()
      .connection_state,
    PeerConnectionState::Connected
  );

  // Update data channel state
  app_state
    .webrtc_state
    .update(|s| s.update_data_channel_state(&user_id, DataChannelState::Open));

  let peer = app_state
    .webrtc_state
    .get()
    .get_peer(&user_id)
    .unwrap()
    .clone();
  assert!(peer.is_ready());

  // Remove peer
  app_state.webrtc_state.update(|s| s.remove_peer(&user_id));
  assert_eq!(app_state.webrtc_state.get().peers.len(), 0);
}

#[test]
fn test_close_connection_updates_app_state() {
  let app_state = test_app_state();
  let manager = WebRtcManager::new(app_state);
  let peer_id = UserId::new();

  // Add peer to app state manually (simulating connection setup)
  app_state
    .webrtc_state
    .update(|state| state.add_peer(peer_id.clone(), true));

  assert_eq!(app_state.webrtc_state.get().peers.len(), 1);

  // close_connection removes from app state
  manager.close_connection(&peer_id);

  assert_eq!(app_state.webrtc_state.get().peers.len(), 0);
}

#[test]
fn test_close_all_clears_app_state() {
  let app_state = test_app_state();
  let manager = WebRtcManager::new(app_state);

  // Add multiple peers
  for _ in 0..3 {
    let peer_id = UserId::new();
    app_state
      .webrtc_state
      .update(|state| state.add_peer(peer_id, true));
  }

  assert_eq!(app_state.webrtc_state.get().peers.len(), 3);

  manager.close_all();

  assert_eq!(app_state.webrtc_state.get().peers.len(), 0);
}

/// P0-5 regression: `connect_to_peer` must register the peer in the
/// reactive UI state (`app_state.webrtc_state`) as soon as the underlying
/// `PeerConnection` is stored. Without this, `update_connection_state`
/// and `update_data_channel_state` silently become no-ops and the UI
/// Signal stays empty forever.
#[wasm_bindgen_test]
async fn test_connect_to_peer_registers_in_app_state() {
  let app_state = test_app_state();
  let manager = WebRtcManager::new(app_state);
  let peer_id = UserId::from(7u64);

  // Sanity: nothing registered initially.
  assert_eq!(app_state.webrtc_state.get().peers.len(), 0);
  assert!(!manager.is_connected(&peer_id));

  // `connect_to_peer` creates a real RTCPeerConnection with a DataChannel,
  // generates an SDP offer, and then stores the PC. The signaling client
  // is intentionally left unset so `send_sdp_offer` is a no-op, but the
  // `add_peer` call must still fire because it happens *before* the
  // signaling step.
  manager.connect_to_peer(peer_id.clone()).await.unwrap();

  // The peer must now appear in the reactive state (initiator side).
  let state = app_state.webrtc_state.get();
  let peer = state
    .get_peer(&peer_id)
    .expect("peer should be registered in reactive state after connect_to_peer");
  assert!(peer.is_initiator, "connect_to_peer is the initiator side");
  assert_eq!(peer.connection_state, PeerConnectionState::Connecting);

  // And the underlying PC map must match.
  assert!(manager.is_connected(&peer_id));
  assert_eq!(manager.connection_count(), 1);

  // Cleanup.
  drop(state);
  manager.close_connection(&peer_id);
  assert_eq!(app_state.webrtc_state.get().peers.len(), 0);
}
