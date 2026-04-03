//! Signal message serialization tests

use message::{signal, types};

#[test]
fn test_signal_register_serialize_roundtrip() {
  let msg = signal::SignalMessage::Register {
    username: "alice".to_string(),
    password: "secret123".to_string(),
  };
  let bytes = bitcode::serialize(&msg).expect("Serialization failed");
  let decoded: signal::SignalMessage =
    bitcode::deserialize(&bytes).expect("Deserialization failed");
  assert_eq!(decoded, msg);
}

#[test]
fn test_signal_sdp_offer_serialize_roundtrip() {
  let msg = signal::SignalMessage::SdpOffer {
    from: "user-1".to_string(),
    to: "user-2".to_string(),
    sdp: "v=0\r\no=- 123 456 IN IP4 0.0.0.0\r\n".to_string(),
  };
  let bytes = bitcode::serialize(&msg).expect("Serialization failed");
  let decoded: signal::SignalMessage =
    bitcode::deserialize(&bytes).expect("Deserialization failed");
  assert_eq!(decoded, msg);
}

#[test]
fn test_signal_ice_candidate_serialize_roundtrip() {
  let msg = signal::SignalMessage::IceCandidate {
    from: "user-1".to_string(),
    to: "user-2".to_string(),
    candidate: "candidate:1 1 UDP 2130706431 192.168.1.1 12345 typ host".to_string(),
  };
  let bytes = bitcode::serialize(&msg).expect("Serialization failed");
  let decoded: signal::SignalMessage =
    bitcode::deserialize(&bytes).expect("Deserialization failed");
  assert_eq!(decoded, msg);
}

#[test]
fn test_signal_connection_invite_serialize_roundtrip() {
  let msg = signal::SignalMessage::ConnectionInvite {
    from: "user-1".to_string(),
    to: "user-2".to_string(),
    message: Some("Want to chat?".to_string()),
    invite_type: signal::InviteType::Chat,
  };
  let bytes = bitcode::serialize(&msg).expect("Serialization failed");
  let decoded: signal::SignalMessage =
    bitcode::deserialize(&bytes).expect("Deserialization failed");
  assert_eq!(decoded, msg);
}

#[test]
fn test_signal_create_room_serialize_roundtrip() {
  let msg = signal::SignalMessage::CreateRoom {
    name: "Test Room".to_string(),
    description: Some("This is a test room".to_string()),
    password: Some("room-pass".to_string()),
    max_members: 10,
    room_type: signal::RoomType::Chat,
  };
  let bytes = bitcode::serialize(&msg).expect("Serialization failed");
  let decoded: signal::SignalMessage =
    bitcode::deserialize(&bytes).expect("Deserialization failed");
  assert_eq!(decoded, msg);
}

#[test]
fn test_signal_theater_control_serialize_roundtrip() {
  let msg = signal::SignalMessage::TheaterControl {
    room_id: "room-1".to_string(),
    action: signal::TheaterAction::Seek(120.5),
  };
  let bytes = bitcode::serialize(&msg).expect("Serialization failed");
  let decoded: signal::SignalMessage =
    bitcode::deserialize(&bytes).expect("Deserialization failed");
  assert_eq!(decoded, msg);
}

#[test]
fn test_signal_ping_pong_serialize_roundtrip() {
  for msg in &[signal::SignalMessage::Ping, signal::SignalMessage::Pong] {
    let bytes = bitcode::serialize(msg).expect("Serialization failed");
    let decoded: signal::SignalMessage =
      bitcode::deserialize(&bytes).expect("Deserialization failed");
    assert_eq!(&decoded, msg);
  }
}

#[test]
fn test_all_signal_variants_serialize() {
  let variants: Vec<signal::SignalMessage> = vec![
    signal::SignalMessage::Register {
      username: "u".to_string(),
      password: "p".to_string(),
    },
    signal::SignalMessage::Login {
      username: "u".to_string(),
      password: "p".to_string(),
    },
    signal::SignalMessage::AuthSuccess {
      user_id: "id".to_string(),
      token: "tok".to_string(),
      username: "u".to_string(),
    },
    signal::SignalMessage::AuthError {
      reason: "err".to_string(),
    },
    signal::SignalMessage::TokenAuth {
      token: "tok".to_string(),
    },
    signal::SignalMessage::SdpOffer {
      from: "a".to_string(),
      to: "b".to_string(),
      sdp: "sdp".to_string(),
    },
    signal::SignalMessage::SdpAnswer {
      from: "a".to_string(),
      to: "b".to_string(),
      sdp: "sdp".to_string(),
    },
    signal::SignalMessage::IceCandidate {
      from: "a".to_string(),
      to: "b".to_string(),
      candidate: "c".to_string(),
    },
    signal::SignalMessage::UserListUpdate { users: vec![] },
    signal::SignalMessage::UserStatusChange {
      user_id: "id".to_string(),
      status: signal::UserStatus::Busy,
    },
    signal::SignalMessage::ConnectionInvite {
      from: "a".to_string(),
      to: "b".to_string(),
      message: None,
      invite_type: signal::InviteType::AudioCall,
    },
    signal::SignalMessage::InviteResponse {
      from: "a".to_string(),
      to: "b".to_string(),
      accepted: true,
    },
    signal::SignalMessage::InviteTimeout {
      from: "a".to_string(),
      to: "b".to_string(),
    },
    signal::SignalMessage::CreateRoom {
      name: "r".to_string(),
      description: None,
      password: None,
      max_members: 8,
      room_type: signal::RoomType::Theater,
    },
    signal::SignalMessage::RoomCreated {
      room_id: "r".to_string(),
    },
    signal::SignalMessage::JoinRoom {
      room_id: "r".to_string(),
      password: None,
    },
    signal::SignalMessage::LeaveRoom {
      room_id: "r".to_string(),
    },
    signal::SignalMessage::RoomMemberUpdate {
      room_id: "r".to_string(),
      members: vec![],
    },
    signal::SignalMessage::RoomListUpdate { rooms: vec![] },
    signal::SignalMessage::RoomError {
      reason: "err".to_string(),
    },
    signal::SignalMessage::KickMember {
      room_id: "r".to_string(),
      target_user_id: "u".to_string(),
    },
    signal::SignalMessage::MuteMember {
      room_id: "r".to_string(),
      target_user_id: "u".to_string(),
      muted: true,
    },
    signal::SignalMessage::MuteAll {
      room_id: "r".to_string(),
      muted: true,
    },
    signal::SignalMessage::TransferOwner {
      room_id: "r".to_string(),
      new_owner_id: "u".to_string(),
    },
    signal::SignalMessage::Kicked {
      room_id: "r".to_string(),
      reason: None,
    },
    signal::SignalMessage::MuteStatusChanged {
      room_id: "r".to_string(),
      muted: false,
    },
    signal::SignalMessage::TheaterControl {
      room_id: "r".to_string(),
      action: signal::TheaterAction::Play,
    },
    signal::SignalMessage::TheaterSync {
      room_id: "r".to_string(),
      current_time: 0.0,
      is_playing: true,
    },
    signal::SignalMessage::CallInvite {
      from: "a".to_string(),
      to: vec!["b".to_string()],
      media_type: types::MediaType::Video,
    },
    signal::SignalMessage::CallResponse {
      from: "a".to_string(),
      to: "b".to_string(),
      accepted: true,
    },
    signal::SignalMessage::CallHangup {
      from: "a".to_string(),
      room_id: None,
    },
    signal::SignalMessage::MediaTrackChanged {
      from: "a".to_string(),
      video_enabled: true,
      audio_enabled: true,
    },
    signal::SignalMessage::CreateInviteLink {
      invite_type: signal::InviteType::Room,
      room_id: Some("r".to_string()),
    },
    signal::SignalMessage::InviteLinkCreated {
      code: "abc123".to_string(),
      expires_at: 1_700_000_000_000,
      invite_type: signal::InviteType::Room,
    },
    signal::SignalMessage::JoinByInviteLink {
      code: "abc123".to_string(),
    },
    signal::SignalMessage::InviteLinkError {
      reason: "expired".to_string(),
    },
    signal::SignalMessage::IceConfig {
      ice_servers: vec!["stun:stun.example.com:3478".to_string()],
    },
    signal::SignalMessage::Ping,
    signal::SignalMessage::Pong,
    signal::SignalMessage::Error {
      code: 404,
      message: "not found".to_string(),
    },
  ];

  for (i, variant) in variants.iter().enumerate() {
    let bytes = bitcode::serialize(variant)
      .unwrap_or_else(|_| panic!("Signal variant {i} serialization failed"));
    let decoded: signal::SignalMessage = bitcode::deserialize(&bytes)
      .unwrap_or_else(|_| panic!("Signal variant {i} deserialization failed"));
    assert_eq!(&decoded, variant, "Signal variant {i} roundtrip mismatch");
  }
}
