//! Moderation action message tests.

use super::*;

#[test]
fn test_mute_member_roundtrip() {
  let msg = MuteMember {
    room_id: RoomId::new(),
    target: UserId::new(),
    duration_secs: Some(300),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: MuteMember = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_ban_member_roundtrip() {
  let msg = BanMember {
    room_id: RoomId::new(),
    target: UserId::new(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: BanMember = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_promote_admin_roundtrip() {
  let msg = PromoteAdmin {
    room_id: RoomId::new(),
    target: UserId::new(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: PromoteAdmin = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_moderation_notification_roundtrip() {
  let msg = ModerationNotification {
    room_id: RoomId::new(),
    action: ModerationAction::Kicked,
    target: UserId::new(),
    reason: Some("Spam".to_string()),
    duration_secs: None,
  };
  let encoded = bitcode::encode(&msg);
  let decoded: ModerationNotification = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_unmute_member_roundtrip() {
  let msg = UnmuteMember {
    room_id: RoomId::new(),
    target: UserId::new(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: UnmuteMember = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_unban_member_roundtrip() {
  let msg = UnbanMember {
    room_id: RoomId::new(),
    target: UserId::new(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: UnbanMember = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_demote_admin_roundtrip() {
  let msg = DemoteAdmin {
    room_id: RoomId::new(),
    target: UserId::new(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: DemoteAdmin = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_theater_mute_all_roundtrip() {
  let msg = TheaterMuteAll {
    room_id: RoomId::new(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: TheaterMuteAll = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_theater_transfer_owner_roundtrip() {
  let msg = TheaterTransferOwner {
    room_id: RoomId::new(),
    target: UserId::new(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: TheaterTransferOwner = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_signaling_message_theater_mute_all_roundtrip() {
  let msg = SignalingMessage::TheaterMuteAll(TheaterMuteAll {
    room_id: RoomId::new(),
  });
  let encoded = bitcode::encode(&msg);
  let decoded: SignalingMessage = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_signaling_message_theater_transfer_owner_roundtrip() {
  let msg = SignalingMessage::TheaterTransferOwner(TheaterTransferOwner {
    room_id: RoomId::new(),
    target: UserId::new(),
  });
  let encoded = bitcode::encode(&msg);
  let decoded: SignalingMessage = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_signaling_message_unmute_member_roundtrip() {
  let msg = SignalingMessage::UnmuteMember(UnmuteMember {
    room_id: RoomId::new(),
    target: UserId::new(),
  });
  let encoded = bitcode::encode(&msg);
  let decoded: SignalingMessage = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_signaling_message_unban_member_roundtrip() {
  let msg = SignalingMessage::UnbanMember(UnbanMember {
    room_id: RoomId::new(),
    target: UserId::new(),
  });
  let encoded = bitcode::encode(&msg);
  let decoded: SignalingMessage = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_signaling_message_demote_admin_roundtrip() {
  let msg = SignalingMessage::DemoteAdmin(DemoteAdmin {
    room_id: RoomId::new(),
    target: UserId::new(),
  });
  let encoded = bitcode::encode(&msg);
  let decoded: SignalingMessage = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_signaling_message_room_announcement_roundtrip() {
  let msg = SignalingMessage::RoomAnnouncement(RoomAnnouncement {
    room_id: RoomId::new(),
    content: "Big announcement!".to_string(),
  });
  let encoded = bitcode::encode(&msg);
  let decoded: SignalingMessage = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_discriminator_theater_messages() {
  let rid = RoomId::new();
  let uid = UserId::new();
  assert_eq!(
    SignalingMessage::TheaterMuteAll(TheaterMuteAll {
      room_id: rid.clone()
    })
    .discriminator(),
    THEATER_MUTE_ALL
  );
  assert_eq!(
    SignalingMessage::TheaterTransferOwner(TheaterTransferOwner {
      room_id: rid,
      target: uid
    })
    .discriminator(),
    THEATER_TRANSFER_OWNER
  );
}

#[test]
fn test_discriminator_moderation_messages() {
  let rid = RoomId::new();
  let uid = UserId::new();
  assert_eq!(
    SignalingMessage::MuteMember(MuteMember {
      room_id: rid.clone(),
      target: uid.clone(),
      duration_secs: None
    })
    .discriminator(),
    MUTE_MEMBER
  );
  assert_eq!(
    SignalingMessage::UnmuteMember(UnmuteMember {
      room_id: rid.clone(),
      target: uid.clone()
    })
    .discriminator(),
    UNMUTE_MEMBER
  );
  assert_eq!(
    SignalingMessage::BanMember(BanMember {
      room_id: rid.clone(),
      target: uid.clone()
    })
    .discriminator(),
    BAN_MEMBER
  );
  assert_eq!(
    SignalingMessage::UnbanMember(UnbanMember {
      room_id: rid.clone(),
      target: uid.clone()
    })
    .discriminator(),
    UNBAN_MEMBER
  );
  assert_eq!(
    SignalingMessage::PromoteAdmin(PromoteAdmin {
      room_id: rid.clone(),
      target: uid.clone()
    })
    .discriminator(),
    PROMOTE_ADMIN
  );
  assert_eq!(
    SignalingMessage::DemoteAdmin(DemoteAdmin {
      room_id: rid.clone(),
      target: uid.clone()
    })
    .discriminator(),
    DEMOTE_ADMIN
  );
  assert_eq!(
    SignalingMessage::NicknameChange(NicknameChange {
      user_id: uid.clone(),
      new_nickname: "nick".into()
    })
    .discriminator(),
    NICKNAME_CHANGE
  );
  assert_eq!(
    SignalingMessage::ModerationNotification(ModerationNotification {
      room_id: rid,
      action: ModerationAction::Kicked,
      target: uid,
      reason: None,
      duration_secs: None
    })
    .discriminator(),
    MODERATION_NOTIFICATION
  );
}
