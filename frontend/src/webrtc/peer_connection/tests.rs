use super::super::types::DataChannelState;
use super::*;

#[test]
fn test_ice_server_config_stun() {
  let config = IceServerConfig::stun("stun:stun.l.google.com:19302");
  assert_eq!(config.url, "stun:stun.l.google.com:19302");
  assert!(config.username.is_none());
  assert!(config.credential.is_none());
}

#[test]
fn test_ice_server_config_turn() {
  let config = IceServerConfig::turn("turn:turn.example.com:3478", "user", "pass");
  assert_eq!(config.url, "turn:turn.example.com:3478");
  assert_eq!(config.username, Some("user".to_string()));
  assert_eq!(config.credential, Some("pass".to_string()));
}

#[test]
fn test_peer_connection_state_from_str() {
  assert_eq!(
    PeerConnectionState::from("connected"),
    PeerConnectionState::Connected
  );
  assert_eq!(
    PeerConnectionState::from("connecting"),
    PeerConnectionState::Connecting
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
  // Unknown state defaults to Closed
  assert_eq!(
    PeerConnectionState::from("unknown"),
    PeerConnectionState::Closed
  );
}

#[test]
fn test_data_channel_state_from_str() {
  assert_eq!(DataChannelState::from("open"), DataChannelState::Open);
  assert_eq!(
    DataChannelState::from("connecting"),
    DataChannelState::Connecting
  );
  assert_eq!(DataChannelState::from("closing"), DataChannelState::Closing);
  assert_eq!(DataChannelState::from("closed"), DataChannelState::Closed);
}
