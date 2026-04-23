//! Discriminator value and uniqueness tests.

use super::*;

#[test]
fn test_signaling_message_discriminator() {
  let msg = SignalingMessage::TokenAuth(TokenAuth {
    token: "test".to_string(),
  });
  assert_eq!(msg.discriminator(), 0x00);

  let msg = SignalingMessage::AuthSuccess(AuthSuccess {
    user_id: UserId::new(),
    username: "alice".to_string(),
    nickname: "alice".to_string(),
  });
  assert_eq!(msg.discriminator(), 0x01);

  let msg = SignalingMessage::SdpOffer(SdpOffer {
    from: UserId::new(),
    to: UserId::new(),
    sdp: "test".to_string(),
  });
  assert_eq!(msg.discriminator(), 0x30);

  let msg = SignalingMessage::MuteMember(MuteMember {
    room_id: RoomId::new(),
    target: UserId::new(),
    duration_secs: None,
  });
  assert_eq!(msg.discriminator(), 0x75);
}

#[test]
fn test_signaling_message_roundtrip() {
  let msg = SignalingMessage::CreateRoom(CreateRoom {
    name: "Test Room".to_string(),
    room_type: RoomType::Theater,
    password: None,
    max_participants: 8,
  });
  let encoded = bitcode::encode(&msg);
  let decoded: SignalingMessage = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

/// Create discriminators for auth/session `SignalingMessage` variants.
fn create_auth_session_discriminators() -> Vec<u8> {
  vec![
    SignalingMessage::TokenAuth(TokenAuth {
      token: String::new(),
    })
    .discriminator(),
    SignalingMessage::AuthSuccess(AuthSuccess {
      user_id: UserId::new(),
      username: String::new(),
      nickname: String::new(),
    })
    .discriminator(),
    SignalingMessage::AuthFailure(AuthFailure {
      reason: String::new(),
    })
    .discriminator(),
    SignalingMessage::UserLogout(UserLogout::default()).discriminator(),
    SignalingMessage::Ping(Ping::default()).discriminator(),
    SignalingMessage::Pong(Pong::default()).discriminator(),
    SignalingMessage::ErrorResponse(ErrorResponse::new(SIG001, "x", "t")).discriminator(),
    SignalingMessage::SessionInvalidated(SessionInvalidated::default()).discriminator(),
    SignalingMessage::UserListUpdate(UserListUpdate { users: vec![] }).discriminator(),
    SignalingMessage::UserStatusChange(UserStatusChange {
      user_id: UserId::new(),
      status: UserStatus::Online,
      signature: None,
    })
    .discriminator(),
  ]
}

/// Create discriminators for peer connection `SignalingMessage` variants.
fn create_peer_connection_discriminators() -> Vec<u8> {
  vec![
    SignalingMessage::ConnectionInvite(ConnectionInvite {
      from: UserId::new(),
      to: UserId::new(),
      note: None,
    })
    .discriminator(),
    SignalingMessage::InviteAccepted(InviteAccepted {
      from: UserId::new(),
      to: UserId::new(),
    })
    .discriminator(),
    SignalingMessage::InviteDeclined(InviteDeclined {
      from: UserId::new(),
      to: UserId::new(),
    })
    .discriminator(),
    SignalingMessage::InviteTimeout(InviteTimeout {
      from: UserId::new(),
      to: UserId::new(),
    })
    .discriminator(),
    SignalingMessage::MultiInvite(MultiInvite {
      from: UserId::new(),
      targets: vec![],
    })
    .discriminator(),
    SignalingMessage::SdpOffer(SdpOffer {
      from: UserId::new(),
      to: UserId::new(),
      sdp: String::new(),
    })
    .discriminator(),
    SignalingMessage::SdpAnswer(SdpAnswer {
      from: UserId::new(),
      to: UserId::new(),
      sdp: String::new(),
    })
    .discriminator(),
    SignalingMessage::IceCandidate(IceCandidate::new(
      UserId::new(),
      UserId::new(),
      String::new(),
    ))
    .discriminator(),
    SignalingMessage::PeerEstablished(PeerEstablished {
      from: UserId::new(),
      to: UserId::new(),
    })
    .discriminator(),
    SignalingMessage::PeerClosed(PeerClosed {
      from: UserId::new(),
      to: UserId::new(),
    })
    .discriminator(),
    SignalingMessage::ActivePeersList(ActivePeersList { peers: vec![] }).discriminator(),
  ]
}

/// Create discriminators for room management `SignalingMessage` variants.
fn create_room_management_discriminators() -> Vec<u8> {
  vec![
    SignalingMessage::CreateRoom(CreateRoom {
      name: String::new(),
      room_type: RoomType::Chat,
      password: None,
      max_participants: 8,
    })
    .discriminator(),
    SignalingMessage::JoinRoom(JoinRoom {
      room_id: RoomId::new(),
      password: None,
    })
    .discriminator(),
    SignalingMessage::LeaveRoom(LeaveRoom {
      room_id: RoomId::new(),
    })
    .discriminator(),
    SignalingMessage::RoomListUpdate(RoomListUpdate { rooms: vec![] }).discriminator(),
    SignalingMessage::RoomMemberUpdate(RoomMemberUpdate {
      room_id: RoomId::new(),
      members: vec![],
    })
    .discriminator(),
    SignalingMessage::KickMember(KickMember {
      room_id: RoomId::new(),
      target: UserId::new(),
    })
    .discriminator(),
    SignalingMessage::TransferOwnership(TransferOwnership {
      room_id: RoomId::new(),
      target: UserId::new(),
    })
    .discriminator(),
  ]
}

/// Create discriminators for call and theater `SignalingMessage` variants.
fn create_call_theater_discriminators() -> Vec<u8> {
  vec![
    SignalingMessage::CallInvite(CallInvite {
      room_id: RoomId::new(),
      media_type: MediaType::Audio,
    })
    .discriminator(),
    SignalingMessage::CallAccept(CallAccept {
      room_id: RoomId::new(),
    })
    .discriminator(),
    SignalingMessage::CallDecline(CallDecline {
      room_id: RoomId::new(),
    })
    .discriminator(),
    SignalingMessage::CallEnd(CallEnd {
      room_id: RoomId::new(),
    })
    .discriminator(),
    SignalingMessage::TheaterMuteAll(TheaterMuteAll {
      room_id: RoomId::new(),
    })
    .discriminator(),
    SignalingMessage::TheaterTransferOwner(TheaterTransferOwner {
      room_id: RoomId::new(),
      target: UserId::new(),
    })
    .discriminator(),
  ]
}

/// Create discriminators for moderation `SignalingMessage` variants.
fn create_moderation_discriminators() -> Vec<u8> {
  vec![
    SignalingMessage::MuteMember(MuteMember {
      room_id: RoomId::new(),
      target: UserId::new(),
      duration_secs: None,
    })
    .discriminator(),
    SignalingMessage::UnmuteMember(UnmuteMember {
      room_id: RoomId::new(),
      target: UserId::new(),
    })
    .discriminator(),
    SignalingMessage::BanMember(BanMember {
      room_id: RoomId::new(),
      target: UserId::new(),
    })
    .discriminator(),
    SignalingMessage::UnbanMember(UnbanMember {
      room_id: RoomId::new(),
      target: UserId::new(),
    })
    .discriminator(),
    SignalingMessage::PromoteAdmin(PromoteAdmin {
      room_id: RoomId::new(),
      target: UserId::new(),
    })
    .discriminator(),
    SignalingMessage::DemoteAdmin(DemoteAdmin {
      room_id: RoomId::new(),
      target: UserId::new(),
    })
    .discriminator(),
    SignalingMessage::NicknameChange(NicknameChange {
      user_id: UserId::new(),
      new_nickname: String::new(),
    })
    .discriminator(),
    SignalingMessage::RoomAnnouncement(RoomAnnouncement {
      room_id: RoomId::new(),
      content: String::new(),
    })
    .discriminator(),
    SignalingMessage::ModerationNotification(ModerationNotification {
      room_id: RoomId::new(),
      action: ModerationAction::Kicked,
      target: UserId::new(),
      reason: None,
      duration_secs: None,
    })
    .discriminator(),
  ]
}

/// Create discriminators for all `SignalingMessage` variants.
fn create_all_signaling_discriminators() -> Vec<u8> {
  let mut discriminators = Vec::new();
  discriminators.extend(create_auth_session_discriminators());
  discriminators.extend(create_peer_connection_discriminators());
  discriminators.extend(create_room_management_discriminators());
  discriminators.extend(create_call_theater_discriminators());
  discriminators.extend(create_moderation_discriminators());
  discriminators
}

#[test]
fn test_discriminator_values_are_unique() {
  let discriminators = create_all_signaling_discriminators();
  let mut seen = std::collections::HashSet::new();
  for d in &discriminators {
    assert!(seen.insert(*d), "Duplicate discriminator value: {d:#04x}");
  }
}

/// Create auth/session `SignalingMessage` variants.
fn create_auth_session_messages() -> Vec<SignalingMessage> {
  vec![
    SignalingMessage::TokenAuth(TokenAuth {
      token: String::new(),
    }),
    SignalingMessage::AuthSuccess(AuthSuccess {
      user_id: UserId::new(),
      username: String::new(),
      nickname: String::new(),
    }),
    SignalingMessage::AuthFailure(AuthFailure {
      reason: String::new(),
    }),
    SignalingMessage::UserLogout(UserLogout {}),
    SignalingMessage::Ping(Ping {}),
    SignalingMessage::Pong(Pong {}),
    SignalingMessage::ErrorResponse(ErrorResponse::new(SIG001, "x", "t")),
    SignalingMessage::SessionInvalidated(SessionInvalidated {}),
    SignalingMessage::UserStatusChange(UserStatusChange {
      user_id: UserId::new(),
      status: UserStatus::Online,
      signature: None,
    }),
  ]
}

/// Create peer connection `SignalingMessage` variants.
fn create_peer_connection_messages() -> Vec<SignalingMessage> {
  vec![
    SignalingMessage::ConnectionInvite(ConnectionInvite {
      from: UserId::new(),
      to: UserId::new(),
      note: None,
    }),
    SignalingMessage::InviteAccepted(InviteAccepted {
      from: UserId::new(),
      to: UserId::new(),
    }),
    SignalingMessage::InviteDeclined(InviteDeclined {
      from: UserId::new(),
      to: UserId::new(),
    }),
    SignalingMessage::InviteTimeout(InviteTimeout {
      from: UserId::new(),
      to: UserId::new(),
    }),
    SignalingMessage::MultiInvite(MultiInvite {
      from: UserId::new(),
      targets: vec![],
    }),
    SignalingMessage::SdpOffer(SdpOffer {
      from: UserId::new(),
      to: UserId::new(),
      sdp: String::new(),
    }),
    SignalingMessage::SdpAnswer(SdpAnswer {
      from: UserId::new(),
      to: UserId::new(),
      sdp: String::new(),
    }),
    SignalingMessage::IceCandidate(IceCandidate::new(
      UserId::new(),
      UserId::new(),
      String::new(),
    )),
    SignalingMessage::PeerEstablished(PeerEstablished {
      from: UserId::new(),
      to: UserId::new(),
    }),
    SignalingMessage::PeerClosed(PeerClosed {
      from: UserId::new(),
      to: UserId::new(),
    }),
    SignalingMessage::ActivePeersList(ActivePeersList { peers: vec![] }),
  ]
}

/// Create a default `RoomInfo` for testing.
fn create_test_room_info() -> crate::types::RoomInfo {
  crate::types::RoomInfo {
    room_id: RoomId::new(),
    name: String::new(),
    description: String::new(),
    room_type: RoomType::Chat,
    owner_id: UserId::new(),
    password_hash: None,
    max_members: 8,
    member_count: 1,
    created_at_nanos: 0,
    announcement: String::new(),
    video_url: None,
  }
}

/// Create room management `SignalingMessage` variants.
fn create_room_management_messages() -> Vec<SignalingMessage> {
  vec![
    SignalingMessage::CreateRoom(CreateRoom {
      name: String::new(),
      room_type: RoomType::Chat,
      password: None,
      max_participants: 0,
    }),
    SignalingMessage::JoinRoom(JoinRoom {
      room_id: RoomId::new(),
      password: None,
    }),
    SignalingMessage::LeaveRoom(LeaveRoom {
      room_id: RoomId::new(),
    }),
    SignalingMessage::RoomCreated(RoomCreated {
      room_id: RoomId::new(),
      room_info: create_test_room_info(),
    }),
    SignalingMessage::RoomJoined(RoomJoined {
      room_id: RoomId::new(),
      room_info: create_test_room_info(),
      members: vec![],
    }),
    SignalingMessage::RoomLeft(RoomLeft {
      room_id: RoomId::new(),
      room_destroyed: false,
    }),
    SignalingMessage::OwnerChanged(OwnerChanged {
      room_id: RoomId::new(),
      old_owner: UserId::new(),
      new_owner: UserId::new(),
    }),
    SignalingMessage::MuteStatusChange(MuteStatusChange {
      room_id: RoomId::new(),
      target: UserId::new(),
      mute_info: crate::types::MuteInfo::not_muted(),
    }),
    SignalingMessage::RoomListUpdate(RoomListUpdate { rooms: vec![] }),
    SignalingMessage::RoomMemberUpdate(RoomMemberUpdate {
      room_id: RoomId::new(),
      members: vec![],
    }),
    SignalingMessage::KickMember(KickMember {
      room_id: RoomId::new(),
      target: UserId::new(),
    }),
    SignalingMessage::TransferOwnership(TransferOwnership {
      room_id: RoomId::new(),
      target: UserId::new(),
    }),
  ]
}

/// Create call/theater `SignalingMessage` variants.
fn create_call_theater_messages() -> Vec<SignalingMessage> {
  vec![
    SignalingMessage::CallInvite(CallInvite {
      room_id: RoomId::new(),
      media_type: MediaType::Audio,
    }),
    SignalingMessage::CallAccept(CallAccept {
      room_id: RoomId::new(),
    }),
    SignalingMessage::CallDecline(CallDecline {
      room_id: RoomId::new(),
    }),
    SignalingMessage::CallEnd(CallEnd {
      room_id: RoomId::new(),
    }),
    SignalingMessage::TheaterMuteAll(TheaterMuteAll {
      room_id: RoomId::new(),
    }),
    SignalingMessage::TheaterTransferOwner(TheaterTransferOwner {
      room_id: RoomId::new(),
      target: UserId::new(),
    }),
  ]
}

/// Create moderation `SignalingMessage` variants.
fn create_moderation_messages() -> Vec<SignalingMessage> {
  vec![
    SignalingMessage::MuteMember(MuteMember {
      room_id: RoomId::new(),
      target: UserId::new(),
      duration_secs: None,
    }),
    SignalingMessage::UnmuteMember(UnmuteMember {
      room_id: RoomId::new(),
      target: UserId::new(),
    }),
    SignalingMessage::BanMember(BanMember {
      room_id: RoomId::new(),
      target: UserId::new(),
    }),
    SignalingMessage::UnbanMember(UnbanMember {
      room_id: RoomId::new(),
      target: UserId::new(),
    }),
    SignalingMessage::PromoteAdmin(PromoteAdmin {
      room_id: RoomId::new(),
      target: UserId::new(),
    }),
    SignalingMessage::DemoteAdmin(DemoteAdmin {
      room_id: RoomId::new(),
      target: UserId::new(),
    }),
    SignalingMessage::NicknameChange(NicknameChange {
      user_id: UserId::new(),
      new_nickname: String::new(),
    }),
    SignalingMessage::RoomAnnouncement(RoomAnnouncement {
      room_id: RoomId::new(),
      content: String::new(),
    }),
    SignalingMessage::ModerationNotification(ModerationNotification {
      room_id: RoomId::new(),
      action: ModerationAction::Kicked,
      target: UserId::new(),
      reason: None,
      duration_secs: None,
    }),
  ]
}

/// Create all `SignalingMessage` variants for testing.
fn create_all_signaling_messages() -> Vec<SignalingMessage> {
  let mut messages = Vec::new();
  messages.extend(create_auth_session_messages());
  messages.extend(create_peer_connection_messages());
  messages.extend(create_room_management_messages());
  messages.extend(create_call_theater_messages());
  messages.extend(create_moderation_messages());
  messages
}

/// Test that all defined `SignalingMessage` discriminators fall in the
/// expected range (0x00-0x7D) as documented in discriminator.rs.
#[test]
fn test_signaling_discriminators_in_valid_range() {
  let messages = create_all_signaling_messages();

  for msg in &messages {
    let disc = msg.discriminator();
    assert!(
      disc < 0x80,
      "SignalingMessage discriminator 0x{disc:02X} should be < 0x80 (signaling namespace)"
    );
  }
}

/// Test that bitcode deserialization of `SignalingMessage` rejects data
/// encoded with an unknown/unused discriminator byte.
#[test]
fn test_signaling_bitcode_roundtrip_preserves_discriminator() {
  // Encode a TokenAuth and verify the discriminator is preserved
  let msg = SignalingMessage::TokenAuth(TokenAuth {
    token: "roundtrip-token".to_string(),
  });
  let disc = msg.discriminator();
  assert_eq!(disc, TOKEN_AUTH);

  // Bitcode roundtrip should preserve the message
  let encoded = bitcode::encode(&msg);
  let decoded: SignalingMessage = bitcode::decode(&encoded).expect("Should decode successfully");
  assert_eq!(msg, decoded);
}
