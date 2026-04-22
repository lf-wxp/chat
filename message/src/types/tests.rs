use super::*;
use chrono::Duration;

#[test]
fn test_user_id_creation() {
  let id1 = UserId::new();
  let id2 = UserId::new();
  assert_ne!(id1, id2);
}

#[test]
fn test_room_id_creation() {
  let id1 = RoomId::new();
  let id2 = RoomId::new();
  assert_ne!(id1, id2);
}

#[test]
fn test_message_id_creation() {
  let id1 = MessageId::new();
  let id2 = MessageId::new();
  assert_ne!(id1, id2);
}

#[test]
fn test_transfer_id_creation() {
  let id1 = TransferId::new();
  let id2 = TransferId::new();
  assert_ne!(id1, id2);
}

#[test]
fn test_user_status_display() {
  assert_eq!(UserStatus::Online.to_string(), "Online");
  assert_eq!(UserStatus::Offline.to_string(), "Offline");
  assert_eq!(UserStatus::Busy.to_string(), "Busy");
  assert_eq!(UserStatus::Away.to_string(), "Away");
}

#[test]
fn test_room_type_display() {
  assert_eq!(RoomType::Chat.to_string(), "Chat");
  assert_eq!(RoomType::Theater.to_string(), "Theater");
}

#[test]
fn test_media_type_display() {
  assert_eq!(MediaType::Audio.to_string(), "Audio");
  assert_eq!(MediaType::Video.to_string(), "Video");
  assert_eq!(MediaType::ScreenShare.to_string(), "Screen Share");
}

#[test]
fn test_message_content_type_display() {
  assert_eq!(MessageContentType::Text.to_string(), "Text");
  assert_eq!(MessageContentType::Sticker.to_string(), "Sticker");
  assert_eq!(MessageContentType::Voice.to_string(), "Voice");
  assert_eq!(MessageContentType::Image.to_string(), "Image");
  assert_eq!(MessageContentType::File.to_string(), "File");
  assert_eq!(MessageContentType::System.to_string(), "System");
}

#[test]
fn test_room_role_ordering() {
  assert!(RoomRole::Owner > RoomRole::Admin);
  assert!(RoomRole::Admin > RoomRole::Member);
  assert!(RoomRole::Owner > RoomRole::Member);
  assert!(RoomRole::Member == RoomRole::Member);
}

#[test]
fn test_mute_info_not_muted() {
  let mute = MuteInfo::not_muted();
  assert!(!mute.is_muted());
}

#[test]
fn test_mute_info_permanent() {
  let mute = MuteInfo::permanent();
  assert!(mute.is_muted());
}

#[test]
fn test_mute_info_timed() {
  // Not yet expired
  let mute = MuteInfo::timed(Duration::hours(1));
  assert!(mute.is_muted());

  // Already expired (negative duration means in the past)
  let past_time = chrono::Utc::now() - Duration::hours(1);
  let expired = MuteInfo::timed_at(past_time);
  assert!(!expired.is_muted());
}

#[test]
fn test_mute_info_expires_at() {
  let not_muted = MuteInfo::not_muted();
  assert!(not_muted.expires_at().is_none());

  let permanent = MuteInfo::permanent();
  assert!(permanent.expires_at().is_none());

  let timed = MuteInfo::timed(Duration::hours(2));
  assert!(timed.expires_at().is_some());
}

#[test]
fn test_user_info_creation() {
  let user_id = UserId::new();
  let user = UserInfo::new(
    user_id.clone(),
    "testuser".to_string(),
    "Test User".to_string(),
  );
  assert_eq!(user.user_id, user_id);
  assert_eq!(user.username, "testuser");
  assert_eq!(user.nickname, "Test User");
  assert_eq!(user.status, UserStatus::Online);
  assert!(user.avatar_url.is_none());
}

#[test]
fn test_user_info_timestamps() {
  let user = UserInfo::new(
    UserId::new(),
    "testuser".to_string(),
    "Test User".to_string(),
  );
  let created = user.created_at();
  let last_seen = user.last_seen();
  // Timestamps should be very close (within 1 second)
  let diff = if created > last_seen {
    created - last_seen
  } else {
    last_seen - created
  };
  assert!(diff < chrono::Duration::seconds(1));
}

#[test]
fn test_room_info_creation() {
  let room_id = RoomId::new();
  let owner_id = UserId::new();
  let room = RoomInfo::new(
    room_id.clone(),
    "Test Room".to_string(),
    RoomType::Chat,
    owner_id.clone(),
  );
  assert_eq!(room.room_id, room_id);
  assert_eq!(room.name, "Test Room");
  assert_eq!(room.room_type, RoomType::Chat);
  assert_eq!(room.owner_id, owner_id);
  assert!(!room.is_password_protected());
  assert!(!room.is_full());
}

#[test]
fn test_room_info_password_protected() {
  let mut room = RoomInfo::new(
    RoomId::new(),
    "Private Room".to_string(),
    RoomType::Chat,
    UserId::new(),
  );
  room.password_hash = Some("hashed_password".to_string());
  assert!(room.is_password_protected());
}

#[test]
fn test_room_info_full() {
  let mut room = RoomInfo::new(
    RoomId::new(),
    "Full Room".to_string(),
    RoomType::Chat,
    UserId::new(),
  );
  room.max_members = 2;
  room.member_count = 2;
  assert!(room.is_full());
}

#[test]
fn test_member_info_creation() {
  let user_id = UserId::new();
  let member = MemberInfo::new(user_id.clone(), "Member Nick".to_string(), RoomRole::Member);
  assert_eq!(member.user_id, user_id);
  assert_eq!(member.nickname, "Member Nick");
  assert_eq!(member.role, RoomRole::Member);
  assert!(!member.is_muted());
}

#[test]
fn test_member_info_muted() {
  let mut member = MemberInfo::new(UserId::new(), "Muted User".to_string(), RoomRole::Member);
  member.mute_info = MuteInfo::permanent();
  assert!(member.is_muted());
}

#[test]
fn test_member_info_touch() {
  let mut member = MemberInfo::new(UserId::new(), "Test".to_string(), RoomRole::Member);
  let original_last_active = member.last_active_nanos;
  std::thread::sleep(std::time::Duration::from_millis(10));
  member.touch();
  assert!(member.last_active_nanos > original_last_active);
}

#[test]
fn test_image_meta_creation() {
  let meta = ImageMeta::new(
    1920,
    1080,
    1_024_000,
    "image/jpeg".to_string(),
    "https://example.com/image.jpg".to_string(),
  );
  assert_eq!(meta.width, 1920);
  assert_eq!(meta.height, 1080);
  assert_eq!(meta.size, 1_024_000);
  assert_eq!(meta.mime_type, "image/jpeg");
  // Use approx comparison for floating point
  let ratio = meta.aspect_ratio();
  let expected = 16.0 / 9.0;
  assert!((ratio - expected).abs() < 0.0001, "aspect ratio mismatch");
}

#[test]
fn test_subtitle_entry_creation() {
  let entry = SubtitleEntry::new(1000, 3000, "Hello, world!".to_string());
  assert_eq!(entry.start_ms, 1000);
  assert_eq!(entry.end_ms, 3000);
  assert_eq!(entry.text, "Hello, world!");
  assert_eq!(entry.duration_ms(), 2000);
  assert!(entry.is_active_at(1500));
  assert!(!entry.is_active_at(500));
  assert!(!entry.is_active_at(4000));
}

#[test]
fn test_network_quality_classification() {
  assert_eq!(
    NetworkQuality::from_metrics(50, 0.5),
    NetworkQuality::Excellent
  );
  assert_eq!(NetworkQuality::from_metrics(150, 2.0), NetworkQuality::Good);
  assert_eq!(NetworkQuality::from_metrics(300, 5.0), NetworkQuality::Fair);
  assert_eq!(
    NetworkQuality::from_metrics(500, 15.0),
    NetworkQuality::Poor
  );
}

#[test]
fn test_network_quality_video_recommendation() {
  assert_eq!(
    NetworkQuality::Excellent.recommended_video_quality(),
    "1080p"
  );
  assert_eq!(NetworkQuality::Good.recommended_video_quality(), "720p");
  assert_eq!(NetworkQuality::Fair.recommended_video_quality(), "480p");
  assert_eq!(NetworkQuality::Poor.recommended_video_quality(), "360p");
}

// ========================================================================
// Serialization Roundtrip Tests
// ========================================================================

fn test_bitcode_roundtrip<
  T: bitcode::Encode + for<'a> bitcode::Decode<'a> + PartialEq + std::fmt::Debug,
>(
  value: &T,
) {
  let encoded = bitcode::encode(value);
  let decoded: T = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(value, &decoded, "Roundtrip failed for {value:?}");
}

#[test]
fn test_user_id_roundtrip() {
  let id = UserId::new();
  test_bitcode_roundtrip(&id);
}

#[test]
fn test_room_id_roundtrip() {
  let id = RoomId::new();
  test_bitcode_roundtrip(&id);
}

#[test]
fn test_transfer_id_roundtrip() {
  let id = TransferId::new();
  test_bitcode_roundtrip(&id);
}

#[test]
fn test_user_status_roundtrip() {
  for status in [
    UserStatus::Online,
    UserStatus::Offline,
    UserStatus::Busy,
    UserStatus::Away,
  ] {
    test_bitcode_roundtrip(&status);
  }
}

#[test]
fn test_room_type_roundtrip() {
  for room_type in [RoomType::Chat, RoomType::Theater] {
    test_bitcode_roundtrip(&room_type);
  }
}

#[test]
fn test_media_type_roundtrip() {
  for media_type in [MediaType::Audio, MediaType::Video, MediaType::ScreenShare] {
    test_bitcode_roundtrip(&media_type);
  }
}

#[test]
fn test_danmaku_position_roundtrip() {
  for position in [
    DanmakuPosition::Scroll,
    DanmakuPosition::Top,
    DanmakuPosition::Bottom,
  ] {
    test_bitcode_roundtrip(&position);
  }
}

#[test]
fn test_message_content_type_roundtrip() {
  for content_type in [
    MessageContentType::Text,
    MessageContentType::Sticker,
    MessageContentType::Voice,
    MessageContentType::Image,
    MessageContentType::File,
    MessageContentType::System,
  ] {
    test_bitcode_roundtrip(&content_type);
  }
}

#[test]
fn test_reaction_action_roundtrip() {
  for action in [ReactionAction::Add, ReactionAction::Remove] {
    test_bitcode_roundtrip(&action);
  }
}

#[test]
fn test_mute_info_roundtrip() {
  let not_muted = MuteInfo::not_muted();
  test_bitcode_roundtrip(&not_muted);

  let timed = MuteInfo::timed(Duration::hours(2));
  test_bitcode_roundtrip(&timed);

  let permanent = MuteInfo::permanent();
  test_bitcode_roundtrip(&permanent);
}

#[test]
fn test_room_role_roundtrip() {
  for role in [RoomRole::Owner, RoomRole::Admin, RoomRole::Member] {
    test_bitcode_roundtrip(&role);
  }
}

#[test]
fn test_network_quality_roundtrip() {
  for quality in [
    NetworkQuality::Excellent,
    NetworkQuality::Good,
    NetworkQuality::Fair,
    NetworkQuality::Poor,
  ] {
    test_bitcode_roundtrip(&quality);
  }
}

#[test]
fn test_user_info_roundtrip() {
  let user = UserInfo::new(
    UserId::new(),
    "testuser".to_string(),
    "Test User".to_string(),
  );
  test_bitcode_roundtrip(&user);
}

#[test]
fn test_room_info_roundtrip() {
  let room = RoomInfo::new(
    RoomId::new(),
    "Test Room".to_string(),
    RoomType::Chat,
    UserId::new(),
  );
  test_bitcode_roundtrip(&room);
}

#[test]
fn test_member_info_roundtrip() {
  let member = MemberInfo::new(UserId::new(), "Test Member".to_string(), RoomRole::Admin);
  test_bitcode_roundtrip(&member);
}

#[test]
fn test_image_meta_roundtrip() {
  let meta = ImageMeta::new(
    1920,
    1080,
    1_024_000,
    "image/jpeg".to_string(),
    "https://example.com/image.jpg".to_string(),
  );
  test_bitcode_roundtrip(&meta);
}

#[test]
fn test_subtitle_entry_roundtrip() {
  let entry = SubtitleEntry::new(1000, 3000, "Hello, world!".to_string());
  test_bitcode_roundtrip(&entry);
}

// ========================================================================
// JSON Serialization Tests (via serde)
// ========================================================================

#[test]
fn test_user_status_json() {
  let status = UserStatus::Online;
  let json = serde_json::to_string(&status).expect("Failed to serialize");
  assert_eq!(json, "\"online\"");

  let decoded: UserStatus = serde_json::from_str(&json).expect("Failed to deserialize");
  assert_eq!(status, decoded);
}

#[test]
fn test_room_type_json() {
  let room_type = RoomType::Theater;
  let json = serde_json::to_string(&room_type).expect("Failed to serialize");
  assert_eq!(json, "\"theater\"");

  let decoded: RoomType = serde_json::from_str(&json).expect("Failed to deserialize");
  assert_eq!(room_type, decoded);
}

#[test]
fn test_mute_info_json() {
  let not_muted = MuteInfo::not_muted();
  let json = serde_json::to_string(&not_muted).expect("Failed to serialize");
  assert!(json.contains("\"type\":\"not_muted\""));

  let timed = MuteInfo::timed(Duration::hours(1));
  let json = serde_json::to_string(&timed).expect("Failed to serialize");
  assert!(json.contains("\"type\":\"timed\""));
  assert!(json.contains("expires_at_nanos"));
}

#[test]
fn test_user_info_json() {
  let user = UserInfo::new(
    UserId::new(),
    "testuser".to_string(),
    "Test User".to_string(),
  );
  let json = serde_json::to_string(&user).expect("Failed to serialize");
  assert!(json.contains("\"username\":\"testuser\""));
  assert!(json.contains("\"nickname\":\"Test User\""));

  let decoded: UserInfo = serde_json::from_str(&json).expect("Failed to deserialize");
  assert_eq!(user.user_id, decoded.user_id);
  assert_eq!(user.username, decoded.username);
  assert_eq!(user.created_at_nanos, decoded.created_at_nanos);
}

#[test]
fn test_room_info_json() {
  let room = RoomInfo::new(
    RoomId::new(),
    "Test Room".to_string(),
    RoomType::Theater,
    UserId::new(),
  );
  let json = serde_json::to_string(&room).expect("Failed to serialize");
  assert!(json.contains("\"name\":\"Test Room\""));
  assert!(json.contains("\"room_type\":\"theater\""));

  let decoded: RoomInfo = serde_json::from_str(&json).expect("Failed to deserialize");
  assert_eq!(room.room_id, decoded.room_id);
  assert_eq!(room.name, decoded.name);
  assert_eq!(room.room_type, decoded.room_type);
}

#[test]
fn test_member_info_json() {
  let member = MemberInfo::new(UserId::new(), "Test Member".to_string(), RoomRole::Admin);
  let json = serde_json::to_string(&member).expect("Failed to serialize");
  assert!(json.contains("\"nickname\":\"Test Member\""));
  assert!(json.contains("\"role\":\"admin\""));

  let decoded: MemberInfo = serde_json::from_str(&json).expect("Failed to deserialize");
  assert_eq!(member.user_id, decoded.user_id);
  assert_eq!(member.nickname, decoded.nickname);
  assert_eq!(member.role, decoded.role);
}

// ===========================================================================
// Identifier JSON Serialization Tests
// ===========================================================================

#[test]
fn test_user_id_json() {
  let id = UserId::new();
  let json = serde_json::to_string(&id).expect("Failed to serialize UserId");
  let decoded: UserId = serde_json::from_str(&json).expect("Failed to deserialize UserId");
  assert_eq!(id, decoded);
}

#[test]
fn test_room_id_json() {
  let id = RoomId::new();
  let json = serde_json::to_string(&id).expect("Failed to serialize RoomId");
  let decoded: RoomId = serde_json::from_str(&json).expect("Failed to deserialize RoomId");
  assert_eq!(id, decoded);
}

#[test]
fn test_message_id_json() {
  let id = MessageId::new();
  let json = serde_json::to_string(&id).expect("Failed to serialize MessageId");
  let decoded: MessageId = serde_json::from_str(&json).expect("Failed to deserialize MessageId");
  assert_eq!(id, decoded);
}

#[test]
fn test_transfer_id_json() {
  let id = TransferId::new();
  let json = serde_json::to_string(&id).expect("Failed to serialize TransferId");
  let decoded: TransferId = serde_json::from_str(&json).expect("Failed to deserialize TransferId");
  assert_eq!(id, decoded);
}

// ===========================================================================
// Structs Edge Case Tests
// ===========================================================================

#[test]
fn test_image_meta_aspect_ratio_zero_height() {
  let meta = ImageMeta::new(
    1920,
    0,
    1024,
    "image/jpeg".to_string(),
    "https://example.com/image.jpg".to_string(),
  );
  assert!(
    (meta.aspect_ratio() - 0.0).abs() < f64::EPSILON,
    "Aspect ratio should be 0 when height is 0"
  );
}

#[test]
fn test_image_meta_aspect_ratio_normal() {
  let meta = ImageMeta::new(
    1920,
    1080,
    1024,
    "image/jpeg".to_string(),
    "https://example.com/image.jpg".to_string(),
  );
  let expected = 1920.0 / 1080.0;
  assert!((meta.aspect_ratio() - expected).abs() < 0.0001);
}

#[test]
fn test_user_info_timestamp_methods() {
  let user = UserInfo::new(
    UserId::new(),
    "testuser".to_string(),
    "Test User".to_string(),
  );
  let created = user.created_at();
  let last_seen = user.last_seen();
  // Timestamps should be very close (within 1 second)
  let diff = if created > last_seen {
    created - last_seen
  } else {
    last_seen - created
  };
  assert!(diff < chrono::Duration::seconds(1));
}

#[test]
fn test_room_info_timestamp_method() {
  let room = RoomInfo::new(
    RoomId::new(),
    "Test Room".to_string(),
    RoomType::Chat,
    UserId::new(),
  );
  let created = room.created_at();
  let now = chrono::Utc::now();
  let diff = if now > created {
    now - created
  } else {
    created - now
  };
  assert!(
    diff < chrono::Duration::seconds(10),
    "Creation time should be very recent"
  );
}

#[test]
fn test_member_info_timestamp_methods() {
  let member = MemberInfo::new(UserId::new(), "Test Member".to_string(), RoomRole::Member);
  let joined = member.joined_at();
  let last_active = member.last_active();
  let now = chrono::Utc::now();

  let joined_diff = if now > joined {
    now - joined
  } else {
    joined - now
  };
  let active_diff = if now > last_active {
    now - last_active
  } else {
    last_active - now
  };

  assert!(joined_diff < chrono::Duration::seconds(10));
  assert!(active_diff < chrono::Duration::seconds(10));
}

// ===========================================================================
// Boundary value tests for max_members
// ===========================================================================

#[test]
fn test_room_info_max_members_default() {
  // Default max_members should be reasonable
  let room = RoomInfo::new(
    RoomId::new(),
    "Test Room".to_string(),
    RoomType::Chat,
    UserId::new(),
  );
  // Default is typically 8 for chat rooms
  assert!(room.max_members >= 2);
  assert!(room.max_members <= 100);
}

#[test]
fn test_room_info_max_members_minimum() {
  // Minimum members should be at least 2 (owner + 1 member)
  let mut room = RoomInfo::new(
    RoomId::new(),
    "Small Room".to_string(),
    RoomType::Chat,
    UserId::new(),
  );
  room.max_members = 2;
  assert_eq!(room.max_members, 2);

  // Room with 1 member (owner) is not full
  room.member_count = 1;
  assert!(!room.is_full());

  // Room with 2 members is full
  room.member_count = 2;
  assert!(room.is_full());
}

#[test]
fn test_room_info_max_members_typical() {
  // Typical max_members value (8)
  let mut room = RoomInfo::new(
    RoomId::new(),
    "Standard Room".to_string(),
    RoomType::Chat,
    UserId::new(),
  );
  room.max_members = 8;

  // Test various member counts
  room.member_count = 1;
  assert!(!room.is_full());

  room.member_count = 7;
  assert!(!room.is_full());

  room.member_count = 8;
  assert!(room.is_full());
}

#[test]
fn test_room_info_max_members_large() {
  // Large room capacity
  let mut room = RoomInfo::new(
    RoomId::new(),
    "Large Room".to_string(),
    RoomType::Chat,
    UserId::new(),
  );
  room.max_members = 50;
  assert_eq!(room.max_members, 50);

  room.member_count = 49;
  assert!(!room.is_full());

  room.member_count = 50;
  assert!(room.is_full());
}

#[test]
fn test_room_info_member_count_edge_cases() {
  let mut room = RoomInfo::new(
    RoomId::new(),
    "Test Room".to_string(),
    RoomType::Chat,
    UserId::new(),
  );
  room.max_members = 8;

  // Member count at boundary
  room.member_count = 0;
  assert!(!room.is_full());

  room.member_count = 1;
  assert!(!room.is_full());

  room.member_count = 8;
  assert!(room.is_full());
}

#[test]
fn test_room_info_theater_vs_chat_capacity() {
  // Theater rooms typically have different capacity expectations
  let chat_room = RoomInfo::new(
    RoomId::new(),
    "Chat Room".to_string(),
    RoomType::Chat,
    UserId::new(),
  );

  let theater_room = RoomInfo::new(
    RoomId::new(),
    "Theater Room".to_string(),
    RoomType::Theater,
    UserId::new(),
  );

  // Both should have reasonable defaults
  assert!(chat_room.max_members >= 2);
  assert!(theater_room.max_members >= 2);
}

// =============================================================================
// Comprehensive Enum Display Tests
// =============================================================================

#[test]
fn test_network_quality_display() {
  assert_eq!(NetworkQuality::Excellent.to_string(), "Excellent");
  assert_eq!(NetworkQuality::Good.to_string(), "Good");
  assert_eq!(NetworkQuality::Fair.to_string(), "Fair");
  assert_eq!(NetworkQuality::Poor.to_string(), "Poor");
}

#[test]
fn test_reaction_action_serialization() {
  // ReactionAction does not implement Display, only test serialization
  let actions = [ReactionAction::Add, ReactionAction::Remove];

  for action in actions {
    let encoded = bitcode::encode(&action);
    let decoded: ReactionAction = bitcode::decode(&encoded).unwrap();
    assert_eq!(action, decoded);
  }
}

#[test]
fn test_network_quality_from_metrics_boundary_rtt() {
  // Test RTT boundaries exactly at thresholds

  // Excellent: rtt < 100ms
  assert_eq!(
    NetworkQuality::from_metrics(99, 0.0),
    NetworkQuality::Excellent
  );
  assert_eq!(
    NetworkQuality::from_metrics(50, 0.0),
    NetworkQuality::Excellent
  );

  // Good: 100 <= rtt < 200
  assert_eq!(NetworkQuality::from_metrics(100, 0.0), NetworkQuality::Good);
  assert_eq!(NetworkQuality::from_metrics(150, 0.0), NetworkQuality::Good);
  assert_eq!(NetworkQuality::from_metrics(199, 0.0), NetworkQuality::Good);

  // Fair: 200 <= rtt < 400
  assert_eq!(NetworkQuality::from_metrics(200, 0.0), NetworkQuality::Fair);
  assert_eq!(NetworkQuality::from_metrics(300, 0.0), NetworkQuality::Fair);
  assert_eq!(NetworkQuality::from_metrics(399, 0.0), NetworkQuality::Fair);

  // Poor: rtt >= 400
  assert_eq!(NetworkQuality::from_metrics(400, 0.0), NetworkQuality::Poor);
  assert_eq!(
    NetworkQuality::from_metrics(1000, 0.0),
    NetworkQuality::Poor
  );
}

#[test]
fn test_network_quality_from_metrics_boundary_packet_loss() {
  // Test packet loss boundaries exactly at thresholds

  // Excellent: loss < 1.0%
  assert_eq!(
    NetworkQuality::from_metrics(50, 0.5),
    NetworkQuality::Excellent
  );
  assert_eq!(
    NetworkQuality::from_metrics(50, 0.99),
    NetworkQuality::Excellent
  );

  // Good: 1.0 <= loss < 3.0
  assert_eq!(NetworkQuality::from_metrics(50, 1.0), NetworkQuality::Good);
  assert_eq!(NetworkQuality::from_metrics(50, 2.0), NetworkQuality::Good);
  assert_eq!(NetworkQuality::from_metrics(50, 2.99), NetworkQuality::Good);

  // Fair: 3.0 <= loss < 10.0
  assert_eq!(NetworkQuality::from_metrics(50, 3.0), NetworkQuality::Fair);
  assert_eq!(NetworkQuality::from_metrics(50, 5.0), NetworkQuality::Fair);
  assert_eq!(NetworkQuality::from_metrics(50, 9.99), NetworkQuality::Fair);

  // Poor: loss >= 10.0
  assert_eq!(NetworkQuality::from_metrics(50, 10.0), NetworkQuality::Poor);
  assert_eq!(NetworkQuality::from_metrics(50, 50.0), NetworkQuality::Poor);
}

#[test]
fn test_network_quality_from_metrics_combined() {
  // Test combined RTT and packet loss scenarios

  // Excellent requires both excellent RTT AND excellent packet loss
  assert_eq!(
    NetworkQuality::from_metrics(99, 0.5),
    NetworkQuality::Excellent
  );

  // If RTT is excellent but packet loss is not, quality is degraded
  assert_eq!(NetworkQuality::from_metrics(50, 1.5), NetworkQuality::Good);
  assert_eq!(NetworkQuality::from_metrics(50, 5.0), NetworkQuality::Fair);
  assert_eq!(NetworkQuality::from_metrics(50, 15.0), NetworkQuality::Poor);

  // If RTT is poor, quality is poor regardless of packet loss
  assert_eq!(NetworkQuality::from_metrics(500, 0.0), NetworkQuality::Poor);
  assert_eq!(NetworkQuality::from_metrics(400, 0.0), NetworkQuality::Poor);
}

#[test]
fn test_network_quality_recommended_video_quality() {
  assert_eq!(
    NetworkQuality::Excellent.recommended_video_quality(),
    "1080p"
  );
  assert_eq!(NetworkQuality::Good.recommended_video_quality(), "720p");
  assert_eq!(NetworkQuality::Fair.recommended_video_quality(), "480p");
  assert_eq!(NetworkQuality::Poor.recommended_video_quality(), "360p");
}

#[test]
fn test_all_enum_defaults() {
  // Verify default values for all enums
  assert_eq!(UserStatus::default(), UserStatus::Online);
  assert_eq!(RoomType::default(), RoomType::Chat);
  assert_eq!(MediaType::default(), MediaType::Audio);
  assert_eq!(DanmakuPosition::default(), DanmakuPosition::Scroll);
  assert_eq!(MessageContentType::default(), MessageContentType::Text);
  assert_eq!(ReactionAction::default(), ReactionAction::Add);
  assert_eq!(NetworkQuality::default(), NetworkQuality::Good);
}

// =============================================================================
// Identifier from_uuid/as_uuid/Display Tests
// =============================================================================

#[test]
fn test_user_id_from_uuid_roundtrip() {
  let uuid = uuid::Uuid::new_v4();
  let user_id = UserId::from_uuid(uuid);
  assert_eq!(*user_id.as_uuid(), uuid);
}

#[test]
fn test_room_id_from_uuid_roundtrip() {
  let uuid = uuid::Uuid::new_v4();
  let room_id = RoomId::from_uuid(uuid);
  assert_eq!(room_id.0, uuid);
}

#[test]
fn test_user_id_display() {
  let user_id = UserId::new();
  let display_str = user_id.to_string();
  // UUID format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx (36 chars)
  assert_eq!(display_str.len(), 36);
  assert!(display_str.contains('-'));
}

#[test]
fn test_room_id_display() {
  let room_id = RoomId::new();
  let display_str = room_id.to_string();
  assert_eq!(display_str.len(), 36);
  assert!(display_str.contains('-'));
}

#[test]
fn test_transfer_id_display() {
  let transfer_id = TransferId::new();
  let display_str = transfer_id.to_string();
  assert_eq!(display_str.len(), 36);
  assert!(display_str.contains('-'));
}

#[test]
fn test_user_id_from_uuid_preserves_value() {
  let specific_uuid = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
  let user_id = UserId::from_uuid(specific_uuid);
  assert_eq!(user_id.as_uuid(), &specific_uuid);
}

#[test]
fn test_room_id_from_uuid_preserves_value() {
  let specific_uuid = uuid::Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap();
  let room_id = RoomId::from_uuid(specific_uuid);
  assert_eq!(room_id.0, specific_uuid);
}

#[test]
fn test_room_id_as_uuid() {
  let uuid = uuid::Uuid::new_v4();
  let room_id = RoomId::from_uuid(uuid);
  assert_eq!(room_id.as_uuid(), &uuid);
}

#[test]
fn test_message_id_from_uuid_roundtrip() {
  let uuid = uuid::Uuid::new_v4();
  let msg_id = MessageId::from_uuid(uuid);
  assert_eq!(msg_id.0, uuid);
}

#[test]
fn test_message_id_as_uuid() {
  let uuid = uuid::Uuid::new_v4();
  let msg_id = MessageId::from_uuid(uuid);
  assert_eq!(msg_id.as_uuid(), &uuid);
}

#[test]
fn test_message_id_display() {
  let msg_id = MessageId::new();
  let display_str = msg_id.to_string();
  assert_eq!(display_str.len(), 36);
  assert!(display_str.contains('-'));
}

#[test]
fn test_message_id_nil() {
  let nil_id = MessageId::nil();
  assert_eq!(nil_id.0, uuid::Uuid::nil());
}

#[test]
fn test_message_id_from_uuid_preserves_value() {
  let specific_uuid = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
  let msg_id = MessageId::from_uuid(specific_uuid);
  assert_eq!(msg_id.as_uuid(), &specific_uuid);
}

#[test]
fn test_transfer_id_from_uuid_roundtrip() {
  let uuid = uuid::Uuid::new_v4();
  let transfer_id = TransferId::from_uuid(uuid);
  assert_eq!(transfer_id.0, uuid);
}

#[test]
fn test_transfer_id_as_uuid() {
  let uuid = uuid::Uuid::new_v4();
  let transfer_id = TransferId::from_uuid(uuid);
  assert_eq!(transfer_id.as_uuid(), &uuid);
}

// =============================================================================
// Struct Method Tests (touch, accessors)
// =============================================================================

#[test]
fn test_user_info_touch_updates_last_seen() {
  let mut user = UserInfo::new(
    UserId::new(),
    "testuser".to_string(),
    "Test User".to_string(),
  );
  let original_last_seen = user.last_seen();
  std::thread::sleep(std::time::Duration::from_millis(10));
  user.touch();
  assert!(user.last_seen() > original_last_seen);
}

#[test]
fn test_room_info_created_at() {
  let before = chrono::Utc::now();
  let room = RoomInfo::new(
    RoomId::new(),
    "Test Room".to_string(),
    RoomType::Chat,
    UserId::new(),
  );
  let after = chrono::Utc::now();

  let created_at = room.created_at();
  assert!(created_at >= before);
  assert!(created_at <= after);
}

#[test]
fn test_room_info_is_full_logic() {
  let mut room = RoomInfo::new(
    RoomId::new(),
    "Test Room".to_string(),
    RoomType::Chat,
    UserId::new(),
  );

  // Initially not full
  assert!(!room.is_full());

  // Set max members to 2 and member_count to 2
  room.max_members = 2;
  room.member_count = 2;
  assert!(room.is_full());

  // Set member_count to 1
  room.member_count = 1;
  assert!(!room.is_full());
}

// =============================================================================
// MuteInfo Edge Case Tests
// =============================================================================

#[test]
fn test_mute_info_serialization_roundtrip() {
  let cases = vec![
    MuteInfo::NotMuted,
    MuteInfo::Permanent,
    MuteInfo::timed(chrono::Duration::seconds(300)),
  ];

  for mute in cases {
    let encoded = bitcode::encode(&mute);
    let decoded: MuteInfo = bitcode::decode(&encoded).unwrap();
    assert_eq!(mute, decoded);
  }
}

#[test]
fn test_mute_info_default() {
  assert_eq!(MuteInfo::default(), MuteInfo::NotMuted);
}

// =============================================================================
// RoomRole Edge Case Tests
// =============================================================================

#[test]
fn test_room_role_display() {
  assert_eq!(RoomRole::Owner.to_string(), "Owner");
  assert_eq!(RoomRole::Admin.to_string(), "Admin");
  assert_eq!(RoomRole::Member.to_string(), "Member");
}

#[test]
fn test_room_role_default() {
  assert_eq!(RoomRole::default(), RoomRole::Member);
}

// =============================================================================
// Identifier Clone Equality Tests (P2-1)
// =============================================================================

#[test]
fn test_user_id_clone_equality() {
  let id = UserId::new();
  let cloned = id.clone();
  assert_eq!(id, cloned);
}

#[test]
fn test_room_id_clone_equality() {
  let id = RoomId::new();
  let cloned = id.clone();
  assert_eq!(id, cloned);
}

#[test]
fn test_transfer_id_clone_equality() {
  let id = TransferId::new();
  let cloned = id.clone();
  assert_eq!(id, cloned);
}

#[test]
fn test_message_id_clone_equality() {
  let id = MessageId::new();
  let cloned = id;
  assert_eq!(id, cloned);
}

// =============================================================================
// Identifier Same-UUID Equality Tests (P2-1)
// =============================================================================

#[test]
fn test_user_id_same_uuid_equality() {
  let uuid = uuid::Uuid::new_v4();
  let id1 = UserId::from_uuid(uuid);
  let id2 = UserId::from_uuid(uuid);
  assert_eq!(id1, id2);
}

#[test]
fn test_room_id_same_uuid_equality() {
  let uuid = uuid::Uuid::new_v4();
  let id1 = RoomId::from_uuid(uuid);
  let id2 = RoomId::from_uuid(uuid);
  assert_eq!(id1, id2);
}

#[test]
fn test_user_id_different_uuid_inequality() {
  let id1 = UserId::new();
  let id2 = UserId::new();
  assert_ne!(id1, id2);
}

#[test]
fn test_room_id_different_uuid_inequality() {
  let id1 = RoomId::new();
  let id2 = RoomId::new();
  assert_ne!(id1, id2);
}

#[test]
fn test_message_id_same_uuid_equality() {
  let uuid = uuid::Uuid::new_v4();
  let id1 = MessageId::from_uuid(uuid);
  let id2 = MessageId::from_uuid(uuid);
  assert_eq!(id1, id2);
}

#[test]
fn test_message_id_different_uuid_inequality() {
  let id1 = MessageId::new();
  let id2 = MessageId::new();
  assert_ne!(id1, id2);
}

// =============================================================================
// Identifier Default Value Validation Tests (P2-1)
// =============================================================================

#[test]
fn test_user_id_default_is_random() {
  // Default UserId uses new() which generates a random UUID
  let id1 = UserId::default();
  let id2 = UserId::default();
  // Two defaults should not be equal (random UUIDs)
  assert_ne!(id1, id2);
}

#[test]
fn test_room_id_default_is_random() {
  let id1 = RoomId::default();
  let id2 = RoomId::default();
  assert_ne!(id1, id2);
}

#[test]
fn test_transfer_id_default_is_random() {
  let id1 = TransferId::default();
  let id2 = TransferId::default();
  assert_ne!(id1, id2);
}

#[test]
fn test_message_id_default_is_random() {
  // Default MessageId uses new() which generates a random UUID
  let id1 = MessageId::default();
  let id2 = MessageId::default();
  // Two defaults should not be equal (random UUIDs)
  assert_ne!(id1, id2);
}

// =============================================================================
// Identifier JSON Serialization with Non-Trivial Values (P2-1)
// =============================================================================

#[test]
fn test_user_id_json_roundtrip_with_specific_uuid() {
  let specific_uuid = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
  let user_id = UserId::from_uuid(specific_uuid);
  let json = serde_json::to_string(&user_id).expect("Failed to serialize");
  let decoded: UserId = serde_json::from_str(&json).expect("Failed to deserialize");
  assert_eq!(user_id, decoded);
}

#[test]
fn test_room_id_json_roundtrip_with_specific_uuid() {
  let specific_uuid = uuid::Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap();
  let room_id = RoomId::from_uuid(specific_uuid);
  let json = serde_json::to_string(&room_id).expect("Failed to serialize");
  let decoded: RoomId = serde_json::from_str(&json).expect("Failed to deserialize");
  assert_eq!(room_id, decoded);
}

#[test]
fn test_transfer_id_json_roundtrip() {
  let transfer_id = TransferId::new();
  let json = serde_json::to_string(&transfer_id).expect("Failed to serialize");
  let decoded: TransferId = serde_json::from_str(&json).expect("Failed to deserialize");
  assert_eq!(transfer_id, decoded);
}

#[test]
fn test_message_id_json_roundtrip_with_specific_uuid() {
  let specific_uuid = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
  let msg_id = MessageId::from_uuid(specific_uuid);
  let json = serde_json::to_string(&msg_id).expect("Failed to serialize");
  let decoded: MessageId = serde_json::from_str(&json).expect("Failed to deserialize");
  assert_eq!(msg_id, decoded);
}

// =============================================================================
// Identifier Equality Transitivity Tests (P2-1)
// =============================================================================

#[test]
fn test_user_id_equality_transitivity() {
  let uuid = uuid::Uuid::new_v4();
  let a = UserId::from_uuid(uuid);
  let b = UserId::from_uuid(uuid);
  let c = a.clone();
  // a == b, b == c => a == c
  assert_eq!(a, b);
  assert_eq!(b, c);
  assert_eq!(a, c);
}

#[test]
fn test_room_id_equality_transitivity() {
  let uuid = uuid::Uuid::new_v4();
  let a = RoomId::from_uuid(uuid);
  let b = RoomId::from_uuid(uuid);
  let c = a.clone();
  assert_eq!(a, b);
  assert_eq!(b, c);
  assert_eq!(a, c);
}

#[test]
fn test_message_id_equality_transitivity() {
  let uuid = uuid::Uuid::new_v4();
  let a = MessageId::from_uuid(uuid);
  let b = MessageId::from_uuid(uuid);
  let c = a;
  assert_eq!(a, b);
  assert_eq!(b, c);
  assert_eq!(a, c);
}

// =============================================================================
// Identifier Hash Consistency Tests (P2-1)
// =============================================================================

#[test]
fn test_user_id_hash_consistency() {
  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};

  let id = UserId::new();
  let mut hasher1 = DefaultHasher::new();
  let mut hasher2 = DefaultHasher::new();
  id.hash(&mut hasher1);
  id.hash(&mut hasher2);
  assert_eq!(hasher1.finish(), hasher2.finish());

  // Clone should have same hash
  let cloned = id.clone();
  let mut hasher3 = DefaultHasher::new();
  cloned.hash(&mut hasher3);
  assert_eq!(hasher1.finish(), hasher3.finish());
}

#[test]
fn test_room_id_hash_consistency() {
  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};

  let id = RoomId::new();
  let mut hasher1 = DefaultHasher::new();
  let mut hasher2 = DefaultHasher::new();
  id.hash(&mut hasher1);
  id.hash(&mut hasher2);
  assert_eq!(hasher1.finish(), hasher2.finish());

  // Clone should have same hash
  let cloned = id.clone();
  let mut hasher3 = DefaultHasher::new();
  cloned.hash(&mut hasher3);
  assert_eq!(hasher1.finish(), hasher3.finish());
}

#[test]
fn test_message_id_hash_consistency() {
  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};

  let id = MessageId::new();
  let mut hasher1 = DefaultHasher::new();
  let mut hasher2 = DefaultHasher::new();
  id.hash(&mut hasher1);
  id.hash(&mut hasher2);
  assert_eq!(hasher1.finish(), hasher2.finish());

  // Copy should have same hash
  let cloned = id;
  let mut hasher3 = DefaultHasher::new();
  cloned.hash(&mut hasher3);
  assert_eq!(hasher1.finish(), hasher3.finish());
}
