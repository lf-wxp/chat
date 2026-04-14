//! Room management message tests.

use super::*;

#[test]
fn test_create_room_roundtrip() {
  let msg = CreateRoom {
    name: "My Room".to_string(),
    room_type: RoomType::Chat,
    password: Some("secret".to_string()),
    max_participants: 8,
  };
  let encoded = bitcode::encode(&msg);
  let decoded: CreateRoom = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_join_room_roundtrip() {
  let msg = JoinRoom {
    room_id: RoomId::new(),
    password: Some("secret".to_string()),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: JoinRoom = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_kick_member_roundtrip() {
  let msg = KickMember {
    room_id: RoomId::new(),
    target: UserId::new(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: KickMember = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_leave_room_roundtrip() {
  let msg = LeaveRoom {
    room_id: RoomId::new(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: LeaveRoom = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_room_list_update_roundtrip() {
  let room1 = RoomInfo::new(
    RoomId::new(),
    "Room A".to_string(),
    RoomType::Chat,
    UserId::new(),
  );
  let room2 = RoomInfo::new(
    RoomId::new(),
    "Room B".to_string(),
    RoomType::Theater,
    UserId::new(),
  );
  let msg = RoomListUpdate {
    rooms: vec![room1, room2],
  };
  let encoded = bitcode::encode(&msg);
  let decoded: RoomListUpdate = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_room_member_update_roundtrip() {
  let member1 = MemberInfo::new(UserId::new(), "Alice".to_string(), RoomRole::Owner);
  let member2 = MemberInfo::new(UserId::new(), "Bob".to_string(), RoomRole::Member);
  let msg = RoomMemberUpdate {
    room_id: RoomId::new(),
    members: vec![member1, member2],
  };
  let encoded = bitcode::encode(&msg);
  let decoded: RoomMemberUpdate = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_transfer_ownership_roundtrip() {
  let msg = TransferOwnership {
    room_id: RoomId::new(),
    target: UserId::new(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: TransferOwnership = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_room_created_roundtrip() {
  let room_info = RoomInfo::new(
    RoomId::new(),
    "New Room".to_string(),
    RoomType::Chat,
    UserId::new(),
  );
  let msg = RoomCreated {
    room_id: RoomId::new(),
    room_info,
  };
  let encoded = bitcode::encode(&msg);
  let decoded: RoomCreated = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_room_joined_roundtrip() {
  let room_info = RoomInfo::new(
    RoomId::new(),
    "Joined Room".to_string(),
    RoomType::Theater,
    UserId::new(),
  );
  let member = MemberInfo::new(UserId::new(), "Player1".to_string(), RoomRole::Member);
  let msg = RoomJoined {
    room_id: RoomId::new(),
    room_info,
    members: vec![member],
  };
  let encoded = bitcode::encode(&msg);
  let decoded: RoomJoined = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_room_left_roundtrip() {
  let msg = RoomLeft {
    room_id: RoomId::new(),
    room_destroyed: true,
  };
  let encoded = bitcode::encode(&msg);
  let decoded: RoomLeft = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_room_left_not_destroyed_roundtrip() {
  let msg = RoomLeft {
    room_id: RoomId::new(),
    room_destroyed: false,
  };
  let encoded = bitcode::encode(&msg);
  let decoded: RoomLeft = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_owner_changed_roundtrip() {
  let msg = OwnerChanged {
    room_id: RoomId::new(),
    old_owner: UserId::new(),
    new_owner: UserId::new(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: OwnerChanged = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_mute_status_change_roundtrip() {
  use crate::types::MuteInfo;
  let msg = MuteStatusChange {
    room_id: RoomId::new(),
    target: UserId::new(),
    mute_info: MuteInfo::permanent(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: MuteStatusChange = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_mute_status_change_not_muted_roundtrip() {
  use crate::types::MuteInfo;
  let msg = MuteStatusChange {
    room_id: RoomId::new(),
    target: UserId::new(),
    mute_info: MuteInfo::not_muted(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: MuteStatusChange = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_mute_status_change_timed_roundtrip() {
  use crate::types::MuteInfo;
  let msg = MuteStatusChange {
    room_id: RoomId::new(),
    target: UserId::new(),
    mute_info: MuteInfo::timed(chrono::Duration::hours(1)),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: MuteStatusChange = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_room_announcement_roundtrip() {
  let msg = RoomAnnouncement {
    room_id: RoomId::new(),
    content: "Welcome to the new room!".to_string(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: RoomAnnouncement = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_signaling_message_leave_room_roundtrip() {
  let msg = SignalingMessage::LeaveRoom(LeaveRoom {
    room_id: RoomId::new(),
  });
  let encoded = bitcode::encode(&msg);
  let decoded: SignalingMessage = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_discriminator_room_management() {
  assert_eq!(
    SignalingMessage::CreateRoom(CreateRoom {
      name: "n".into(),
      room_type: RoomType::Chat,
      password: None,
      max_participants: 8
    })
    .discriminator(),
    CREATE_ROOM
  );
  let rid = RoomId::new();
  assert_eq!(
    SignalingMessage::JoinRoom(JoinRoom {
      room_id: rid.clone(),
      password: None
    })
    .discriminator(),
    JOIN_ROOM
  );
  assert_eq!(
    SignalingMessage::LeaveRoom(LeaveRoom {
      room_id: rid.clone()
    })
    .discriminator(),
    LEAVE_ROOM
  );
  assert_eq!(
    SignalingMessage::RoomListUpdate(RoomListUpdate { rooms: vec![] }).discriminator(),
    ROOM_LIST_UPDATE
  );
  assert_eq!(
    SignalingMessage::RoomMemberUpdate(RoomMemberUpdate {
      room_id: rid.clone(),
      members: vec![]
    })
    .discriminator(),
    ROOM_MEMBER_UPDATE
  );
  let uid = UserId::new();
  assert_eq!(
    SignalingMessage::KickMember(KickMember {
      room_id: rid.clone(),
      target: uid.clone()
    })
    .discriminator(),
    KICK_MEMBER
  );
  assert_eq!(
    SignalingMessage::TransferOwnership(TransferOwnership {
      room_id: rid,
      target: uid
    })
    .discriminator(),
    TRANSFER_OWNERSHIP
  );
}

#[test]
fn test_discriminator_room_response_messages() {
  let rid = RoomId::new();
  let uid = UserId::new();
  let room_info = RoomInfo::new(rid.clone(), "n".into(), RoomType::Chat, uid.clone());
  let member_info = MemberInfo::new(uid.clone(), "n".into(), RoomRole::Member);
  assert_eq!(
    SignalingMessage::RoomCreated(RoomCreated {
      room_id: rid.clone(),
      room_info: room_info.clone()
    })
    .discriminator(),
    ROOM_CREATED
  );
  assert_eq!(
    SignalingMessage::RoomJoined(RoomJoined {
      room_id: rid.clone(),
      room_info: room_info.clone(),
      members: vec![member_info.clone()]
    })
    .discriminator(),
    ROOM_JOINED
  );
  assert_eq!(
    SignalingMessage::RoomLeft(RoomLeft {
      room_id: rid.clone(),
      room_destroyed: false
    })
    .discriminator(),
    ROOM_LEFT
  );
  assert_eq!(
    SignalingMessage::OwnerChanged(OwnerChanged {
      room_id: rid.clone(),
      old_owner: uid.clone(),
      new_owner: UserId::new()
    })
    .discriminator(),
    OWNER_CHANGED
  );
  assert_eq!(
    SignalingMessage::MuteStatusChange(MuteStatusChange {
      room_id: rid,
      target: uid,
      mute_info: crate::types::MuteInfo::Permanent
    })
    .discriminator(),
    MUTE_STATUS_CHANGE
  );
}
