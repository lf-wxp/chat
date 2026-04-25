//! Struct type tests: `UserInfo`, `RoomInfo`, `MemberInfo`, `ImageMeta`.

use super::*;

// ===========================================================================
// UserInfo tests
// ===========================================================================

#[test]
fn test_user_info_creation() {
  let user_id = UserId::new();
  let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
  let user_info = UserInfo {
    user_id: user_id.clone(),
    username: "alice".to_string(),
    nickname: "Alice".to_string(),
    status: UserStatus::Online,
    avatar_url: None,
    bio: String::new(),
    created_at_nanos: now,
    last_seen_nanos: now,
  };

  assert_eq!(user_info.user_id, user_id);
  assert_eq!(user_info.username, "alice");
  assert_eq!(user_info.nickname, "Alice");
  assert!(user_info.avatar_url.is_none());
  assert_eq!(user_info.status, UserStatus::Online);
  assert!(user_info.bio.is_empty());
}

#[test]
fn test_user_info_with_avatar() {
  let user_id = UserId::new();
  let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
  let user_info = UserInfo {
    user_id,
    username: "bob".to_string(),
    nickname: "Bob".to_string(),
    avatar_url: Some("https://example.com/avatar.png".to_string()),
    status: UserStatus::Away,
    bio: "Hello world".to_string(),
    created_at_nanos: now,
    last_seen_nanos: now,
  };

  assert!(user_info.avatar_url.is_some());
  assert_eq!(
    user_info.avatar_url.unwrap(),
    "https://example.com/avatar.png"
  );
  assert_eq!(user_info.bio, "Hello world");
}

#[test]
fn test_user_info_default_status() {
  let user_id = UserId::new();
  let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
  let user_info = UserInfo {
    user_id,
    username: "charlie".to_string(),
    nickname: "Charlie".to_string(),
    status: UserStatus::default(),
    avatar_url: None,
    bio: String::new(),
    created_at_nanos: now,
    last_seen_nanos: now,
  };

  assert_eq!(user_info.status, UserStatus::Online);
}

// ===========================================================================
// RoomInfo tests
// ===========================================================================

#[test]
fn test_room_info_creation() {
  let room_id = RoomId::new();
  let owner_id = UserId::new();
  let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
  let room_info = RoomInfo {
    room_id: room_id.clone(),
    name: "Test Room".to_string(),
    description: String::new(),
    owner_id: owner_id.clone(),
    room_type: RoomType::Chat,
    password_hash: None,
    max_members: 8,
    member_count: 1,
    created_at_nanos: now,
    announcement: String::new(),
    video_url: None,
  };

  assert_eq!(room_info.room_id, room_id);
  assert_eq!(room_info.name, "Test Room");
  assert_eq!(room_info.owner_id, owner_id);
  assert_eq!(room_info.room_type, RoomType::Chat);
  assert_eq!(room_info.member_count, 1);
  assert_eq!(room_info.max_members, 8);
  assert!(room_info.password_hash.is_none());
  assert!(!room_info.is_password_protected());
}

#[test]
fn test_room_info_theater_type() {
  let room_id = RoomId::new();
  let owner_id = UserId::new();
  let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
  let room_info = RoomInfo {
    room_id,
    name: "Theater Room".to_string(),
    description: "A theater room".to_string(),
    owner_id,
    room_type: RoomType::Theater,
    password_hash: Some("hashed_password".to_string()),
    max_members: 50,
    member_count: 5,
    created_at_nanos: now,
    announcement: "Welcome!".to_string(),
    video_url: Some("https://example.com/video.mp4".to_string()),
  };

  assert_eq!(room_info.room_type, RoomType::Theater);
  assert!(room_info.is_password_protected());
  assert_eq!(
    room_info.video_url.unwrap(),
    "https://example.com/video.mp4"
  );
  assert_eq!(room_info.announcement, "Welcome!");
}

#[test]
fn test_room_info_password_protection() {
  let room_id = RoomId::new();
  let owner_id = UserId::new();
  let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
  let room_info_no_pwd = RoomInfo {
    room_id,
    name: "No Password".to_string(),
    description: String::new(),
    owner_id,
    room_type: RoomType::Chat,
    password_hash: None,
    max_members: 8,
    member_count: 0,
    created_at_nanos: now,
    announcement: String::new(),
    video_url: None,
  };

  assert!(!room_info_no_pwd.is_password_protected());
}

// ===========================================================================
// MemberInfo tests
// ===========================================================================

#[test]
fn test_member_info_creation() {
  let user_id = UserId::new();
  let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
  let member_info = MemberInfo {
    user_id: user_id.clone(),
    nickname: "Alice".to_string(),
    role: RoomRole::Member,
    mute_info: MuteInfo::NotMuted,
    joined_at_nanos: now,
    last_active_nanos: now,
  };

  assert_eq!(member_info.user_id, user_id);
  assert_eq!(member_info.nickname, "Alice");
  assert_eq!(member_info.role, RoomRole::Member);
  assert!(!member_info.is_muted());
}

#[test]
fn test_member_info_as_owner() {
  let user_id = UserId::new();
  let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
  let member_info = MemberInfo {
    user_id,
    nickname: "Owner".to_string(),
    role: RoomRole::Owner,
    mute_info: MuteInfo::NotMuted,
    joined_at_nanos: now,
    last_active_nanos: now,
  };

  assert_eq!(member_info.role, RoomRole::Owner);
  assert!(!member_info.is_muted());
}

#[test]
fn test_member_info_as_admin() {
  let user_id = UserId::new();
  let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
  let member_info = MemberInfo {
    user_id,
    nickname: "Admin".to_string(),
    role: RoomRole::Admin,
    mute_info: MuteInfo::NotMuted,
    joined_at_nanos: now,
    last_active_nanos: now,
  };

  assert_eq!(member_info.role, RoomRole::Admin);
}

#[test]
fn test_member_info_muted_permanent() {
  let user_id = UserId::new();
  let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
  let member_info = MemberInfo {
    user_id,
    nickname: "MutedUser".to_string(),
    role: RoomRole::Member,
    mute_info: MuteInfo::Permanent,
    joined_at_nanos: now,
    last_active_nanos: now,
  };

  assert!(member_info.is_muted());
}

#[test]
fn test_member_info_muted_timed() {
  let user_id = UserId::new();
  let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
  let member_info = MemberInfo {
    user_id,
    nickname: "TimedMuted".to_string(),
    role: RoomRole::Member,
    mute_info: MuteInfo::timed(chrono::Duration::minutes(5)),
    joined_at_nanos: now,
    last_active_nanos: now,
  };

  assert!(member_info.is_muted());
}

// ===========================================================================
// ImageMeta tests
// ===========================================================================

#[test]
fn test_image_meta_creation() {
  let meta = ImageMeta {
    width: 800,
    height: 600,
    size: 1024,
    mime_type: "image/png".to_string(),
    thumbnail_url: None,
    original_url: "https://example.com/image.png".to_string(),
  };

  assert_eq!(meta.width, 800);
  assert_eq!(meta.height, 600);
  assert_eq!(meta.mime_type, "image/png");
  assert_eq!(meta.size, 1024);
  assert!(meta.thumbnail_url.is_none());
  assert_eq!(meta.original_url, "https://example.com/image.png");
}

#[test]
fn test_image_meta_jpeg_with_thumbnail() {
  let meta = ImageMeta {
    width: 1920,
    height: 1080,
    size: 204_800,
    mime_type: "image/jpeg".to_string(),
    thumbnail_url: Some("https://example.com/thumb.jpg".to_string()),
    original_url: "https://example.com/photo.jpg".to_string(),
  };

  assert_eq!(meta.mime_type, "image/jpeg");
  assert_eq!(meta.width, 1920);
  assert_eq!(meta.height, 1080);
  assert!(meta.thumbnail_url.is_some());
}
