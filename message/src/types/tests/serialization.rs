//! Serialization roundtrip tests: bitcode and JSON.

use super::*;

// ===========================================================================
// Identifier serialization (bitcode)
// ===========================================================================

#[test]
fn test_user_id_bitcode_roundtrip() {
  let id = UserId::new();
  let encoded = bitcode::encode(&id);
  let decoded: UserId = bitcode::decode(&encoded).unwrap();
  assert_eq!(id, decoded);
}

#[test]
fn test_room_id_bitcode_roundtrip() {
  let id = RoomId::new();
  let encoded = bitcode::encode(&id);
  let decoded: RoomId = bitcode::decode(&encoded).unwrap();
  assert_eq!(id, decoded);
}

#[test]
fn test_message_id_bitcode_roundtrip() {
  let id = MessageId::new();
  let encoded = bitcode::encode(&id);
  let decoded: MessageId = bitcode::decode(&encoded).unwrap();
  assert_eq!(id, decoded);
}

#[test]
fn test_transfer_id_bitcode_roundtrip() {
  let id = TransferId::new();
  let encoded = bitcode::encode(&id);
  let decoded: TransferId = bitcode::decode(&encoded).unwrap();
  assert_eq!(id, decoded);
}

// ===========================================================================
// Enum serialization (bitcode)
// ===========================================================================

#[test]
fn test_room_type_bitcode_roundtrip() {
  for variant in [RoomType::Chat, RoomType::Theater] {
    let encoded = bitcode::encode(&variant);
    let decoded: RoomType = bitcode::decode(&encoded).unwrap();
    assert_eq!(variant, decoded);
  }
}

#[test]
fn test_user_status_bitcode_roundtrip() {
  for variant in [
    UserStatus::Online,
    UserStatus::Offline,
    UserStatus::Away,
    UserStatus::Busy,
  ] {
    let encoded = bitcode::encode(&variant);
    let decoded: UserStatus = bitcode::decode(&encoded).unwrap();
    assert_eq!(variant, decoded);
  }
}

#[test]
fn test_network_quality_bitcode_roundtrip() {
  for variant in [
    NetworkQuality::Excellent,
    NetworkQuality::Good,
    NetworkQuality::Fair,
    NetworkQuality::Poor,
  ] {
    let encoded = bitcode::encode(&variant);
    let decoded: NetworkQuality = bitcode::decode(&encoded).unwrap();
    assert_eq!(variant, decoded);
  }
}

#[test]
fn test_message_content_type_bitcode_roundtrip() {
  for variant in [
    MessageContentType::Text,
    MessageContentType::Image,
    MessageContentType::System,
  ] {
    let encoded = bitcode::encode(&variant);
    let decoded: MessageContentType = bitcode::decode(&encoded).unwrap();
    assert_eq!(variant, decoded);
  }
}

#[test]
fn test_media_type_bitcode_roundtrip() {
  for variant in [MediaType::Audio, MediaType::Video, MediaType::ScreenShare] {
    let encoded = bitcode::encode(&variant);
    let decoded: MediaType = bitcode::decode(&encoded).unwrap();
    assert_eq!(variant, decoded);
  }
}

#[test]
fn test_danmaku_position_bitcode_roundtrip() {
  for variant in [
    DanmakuPosition::Scroll,
    DanmakuPosition::Top,
    DanmakuPosition::Bottom,
  ] {
    let encoded = bitcode::encode(&variant);
    let decoded: DanmakuPosition = bitcode::decode(&encoded).unwrap();
    assert_eq!(variant, decoded);
  }
}

#[test]
fn test_reaction_action_bitcode_roundtrip() {
  for variant in [ReactionAction::Add, ReactionAction::Remove] {
    let encoded = bitcode::encode(&variant);
    let decoded: ReactionAction = bitcode::decode(&encoded).unwrap();
    assert_eq!(variant, decoded);
  }
}

#[test]
fn test_room_role_bitcode_roundtrip() {
  for variant in [RoomRole::Owner, RoomRole::Admin, RoomRole::Member] {
    let encoded = bitcode::encode(&variant);
    let decoded: RoomRole = bitcode::decode(&encoded).unwrap();
    assert_eq!(variant, decoded);
  }
}

// ===========================================================================
// MuteInfo serialization
// ===========================================================================

#[test]
fn test_mute_info_not_muted_bitcode_roundtrip() {
  let mute = MuteInfo::NotMuted;
  let encoded = bitcode::encode(&mute);
  let decoded: MuteInfo = bitcode::decode(&encoded).unwrap();
  assert_eq!(mute, decoded);
}

#[test]
fn test_mute_info_permanent_bitcode_roundtrip() {
  let mute = MuteInfo::Permanent;
  let encoded = bitcode::encode(&mute);
  let decoded: MuteInfo = bitcode::decode(&encoded).unwrap();
  assert_eq!(mute, decoded);
}

#[test]
fn test_mute_info_timed_bitcode_roundtrip() {
  let mute = MuteInfo::timed(chrono::Duration::minutes(5));
  let encoded = bitcode::encode(&mute);
  let decoded: MuteInfo = bitcode::decode(&encoded).unwrap();
  assert_eq!(mute, decoded);
}

#[test]
fn test_mute_info_not_muted_json_roundtrip() {
  let mute = MuteInfo::NotMuted;
  let json = serde_json::to_string(&mute).unwrap();
  let decoded: MuteInfo = serde_json::from_str(&json).unwrap();
  assert_eq!(mute, decoded);
}

#[test]
fn test_mute_info_permanent_json_roundtrip() {
  let mute = MuteInfo::Permanent;
  let json = serde_json::to_string(&mute).unwrap();
  let decoded: MuteInfo = serde_json::from_str(&json).unwrap();
  assert_eq!(mute, decoded);
}

#[test]
fn test_mute_info_timed_json_roundtrip() {
  let mute = MuteInfo::timed(chrono::Duration::minutes(10));
  let json = serde_json::to_string(&mute).unwrap();
  let decoded: MuteInfo = serde_json::from_str(&json).unwrap();
  assert_eq!(mute, decoded);
}

// ===========================================================================
// Struct serialization (bitcode)
// ===========================================================================

#[test]
fn test_user_info_bitcode_roundtrip() {
  let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
  let user_info = UserInfo {
    user_id: UserId::new(),
    username: "alice".to_string(),
    nickname: "Alice".to_string(),
    avatar_url: Some("https://example.com/avatar.png".to_string()),
    status: UserStatus::Online,
    bio: "Hello".to_string(),
    created_at_nanos: now,
    last_seen_nanos: now,
  };

  let encoded = bitcode::encode(&user_info);
  let decoded: UserInfo = bitcode::decode(&encoded).unwrap();
  assert_eq!(user_info.user_id, decoded.user_id);
  assert_eq!(user_info.username, decoded.username);
  assert_eq!(user_info.nickname, decoded.nickname);
  assert_eq!(user_info.avatar_url, decoded.avatar_url);
  assert_eq!(user_info.status, decoded.status);
  assert_eq!(user_info.bio, decoded.bio);
}

#[test]
fn test_room_info_bitcode_roundtrip() {
  let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
  let room_info = RoomInfo {
    room_id: RoomId::new(),
    name: "Test Room".to_string(),
    description: "A test room".to_string(),
    owner_id: UserId::new(),
    room_type: RoomType::Chat,
    password_hash: Some("hashed".to_string()),
    max_members: 8,
    member_count: 3,
    created_at_nanos: now,
    announcement: "Welcome!".to_string(),
    video_url: None,
  };

  let encoded = bitcode::encode(&room_info);
  let decoded: RoomInfo = bitcode::decode(&encoded).unwrap();
  assert_eq!(room_info.room_id, decoded.room_id);
  assert_eq!(room_info.name, decoded.name);
  assert_eq!(room_info.owner_id, decoded.owner_id);
  assert_eq!(room_info.room_type, decoded.room_type);
  assert_eq!(room_info.member_count, decoded.member_count);
  assert_eq!(room_info.max_members, decoded.max_members);
  assert_eq!(room_info.password_hash, decoded.password_hash);
  assert_eq!(room_info.announcement, decoded.announcement);
}

#[test]
fn test_member_info_bitcode_roundtrip() {
  let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
  let member_info = MemberInfo {
    user_id: UserId::new(),
    nickname: "Bob".to_string(),
    role: RoomRole::Admin,
    mute_info: MuteInfo::Permanent,
    joined_at_nanos: now,
    last_active_nanos: now,
  };

  let encoded = bitcode::encode(&member_info);
  let decoded: MemberInfo = bitcode::decode(&encoded).unwrap();
  assert_eq!(member_info.user_id, decoded.user_id);
  assert_eq!(member_info.nickname, decoded.nickname);
  assert_eq!(member_info.role, decoded.role);
  assert_eq!(member_info.mute_info, decoded.mute_info);
  assert_eq!(member_info.joined_at_nanos, decoded.joined_at_nanos);
}

#[test]
fn test_image_meta_bitcode_roundtrip() {
  let meta = ImageMeta {
    width: 800,
    height: 600,
    size: 1024,
    mime_type: "image/png".to_string(),
    thumbnail_url: Some("https://example.com/thumb.png".to_string()),
    original_url: "https://example.com/image.png".to_string(),
  };

  let encoded = bitcode::encode(&meta);
  let decoded: ImageMeta = bitcode::decode(&encoded).unwrap();
  assert_eq!(meta.width, decoded.width);
  assert_eq!(meta.height, decoded.height);
  assert_eq!(meta.mime_type, decoded.mime_type);
  assert_eq!(meta.size, decoded.size);
  assert_eq!(meta.thumbnail_url, decoded.thumbnail_url);
  assert_eq!(meta.original_url, decoded.original_url);
}

// ===========================================================================
// JSON serialization
// ===========================================================================

#[test]
fn test_user_id_json_roundtrip() {
  let id = UserId::new();
  let json = serde_json::to_string(&id).unwrap();
  let decoded: UserId = serde_json::from_str(&json).unwrap();
  assert_eq!(id, decoded);
}

#[test]
fn test_room_id_json_roundtrip() {
  let id = RoomId::new();
  let json = serde_json::to_string(&id).unwrap();
  let decoded: RoomId = serde_json::from_str(&json).unwrap();
  assert_eq!(id, decoded);
}

#[test]
fn test_message_id_json_roundtrip() {
  let id = MessageId::new();
  let json = serde_json::to_string(&id).unwrap();
  let decoded: MessageId = serde_json::from_str(&json).unwrap();
  assert_eq!(id, decoded);
}

#[test]
fn test_user_info_json_roundtrip() {
  let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
  let user_info = UserInfo {
    user_id: UserId::new(),
    username: "alice".to_string(),
    nickname: "Alice".to_string(),
    avatar_url: None,
    status: UserStatus::Online,
    bio: String::new(),
    created_at_nanos: now,
    last_seen_nanos: now,
  };

  let json = serde_json::to_string(&user_info).unwrap();
  let decoded: UserInfo = serde_json::from_str(&json).unwrap();
  assert_eq!(user_info.user_id, decoded.user_id);
  assert_eq!(user_info.username, decoded.username);
  assert_eq!(user_info.nickname, decoded.nickname);
  assert_eq!(user_info.status, decoded.status);
}

#[test]
fn test_room_info_json_roundtrip() {
  let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
  let room_info = RoomInfo {
    room_id: RoomId::new(),
    name: "Chat Room".to_string(),
    description: String::new(),
    owner_id: UserId::new(),
    room_type: RoomType::Chat,
    password_hash: None,
    max_members: 8,
    member_count: 5,
    created_at_nanos: now,
    announcement: String::new(),
    video_url: None,
  };

  let json = serde_json::to_string(&room_info).unwrap();
  let decoded: RoomInfo = serde_json::from_str(&json).unwrap();
  assert_eq!(room_info.room_id, decoded.room_id);
  assert_eq!(room_info.name, decoded.name);
  assert_eq!(room_info.room_type, decoded.room_type);
}

#[test]
fn test_member_info_json_roundtrip() {
  let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
  let member_info = MemberInfo {
    user_id: UserId::new(),
    nickname: "Charlie".to_string(),
    role: RoomRole::Member,
    mute_info: MuteInfo::NotMuted,
    joined_at_nanos: now,
    last_active_nanos: now,
  };

  let json = serde_json::to_string(&member_info).unwrap();
  let decoded: MemberInfo = serde_json::from_str(&json).unwrap();
  assert_eq!(member_info.user_id, decoded.user_id);
  assert_eq!(member_info.nickname, decoded.nickname);
  assert_eq!(member_info.role, decoded.role);
}

#[test]
fn test_room_type_json_roundtrip() {
  for variant in [RoomType::Chat, RoomType::Theater] {
    let json = serde_json::to_string(&variant).unwrap();
    let decoded: RoomType = serde_json::from_str(&json).unwrap();
    assert_eq!(variant, decoded);
  }
}

#[test]
fn test_user_status_json_roundtrip() {
  for variant in [
    UserStatus::Online,
    UserStatus::Offline,
    UserStatus::Away,
    UserStatus::Busy,
  ] {
    let json = serde_json::to_string(&variant).unwrap();
    let decoded: UserStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(variant, decoded);
  }
}

#[test]
fn test_network_quality_json_roundtrip() {
  for variant in [
    NetworkQuality::Excellent,
    NetworkQuality::Good,
    NetworkQuality::Fair,
    NetworkQuality::Poor,
  ] {
    let json = serde_json::to_string(&variant).unwrap();
    let decoded: NetworkQuality = serde_json::from_str(&json).unwrap();
    assert_eq!(variant, decoded);
  }
}

// ===========================================================================
// MuteInfo JSON serialization (duplicates above - kept for completeness)
// ===========================================================================

// test_mute_info_permanent_bitcode_roundtrip, test_mute_info_timed_bitcode_roundtrip,
// test_mute_info_permanent_json_roundtrip, and test_mute_info_timed_json_roundtrip
// are already defined in the MuteInfo serialization section above.
