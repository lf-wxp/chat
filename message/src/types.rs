//! Core data types for WebRTC Chat Application.
//!
//! This module defines all fundamental types used throughout the application,
//! including identifiers, enums, and structs for users, rooms, and messages.

use bitcode::{Decode, Encode};
use chrono::{DateTime, Duration, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

// ============================================================================
// Identifier Types
// ============================================================================

/// Unique identifier for a user.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode)]
pub struct UserId(pub Uuid);

impl UserId {
  /// Create a new random `UserId`.
  #[must_use]
  pub fn new() -> Self {
    Self(Uuid::new_v4())
  }

  /// Create a `UserId` from a `Uuid`.
  #[must_use]
  pub const fn from_uuid(uuid: Uuid) -> Self {
    Self(uuid)
  }

  /// Get the inner `Uuid`.
  #[must_use]
  pub const fn as_uuid(&self) -> &Uuid {
    &self.0
  }
}

impl Default for UserId {
  fn default() -> Self {
    Self::new()
  }
}

impl fmt::Display for UserId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

/// Unique identifier for a room.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode)]
pub struct RoomId(pub Uuid);

impl RoomId {
  /// Create a new random `RoomId`.
  #[must_use]
  pub fn new() -> Self {
    Self(Uuid::new_v4())
  }

  /// Create a `RoomId` from a `Uuid`.
  #[must_use]
  pub const fn from_uuid(uuid: Uuid) -> Self {
    Self(uuid)
  }
}

impl Default for RoomId {
  fn default() -> Self {
    Self::new()
  }
}

impl fmt::Display for RoomId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

/// Unique identifier for a message.
pub type MessageId = Uuid;

/// Unique identifier for a file transfer.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode)]
pub struct TransferId(pub Uuid);

impl TransferId {
  /// Create a new random `TransferId`.
  #[must_use]
  pub fn new() -> Self {
    Self(Uuid::new_v4())
  }
}

impl Default for TransferId {
  fn default() -> Self {
    Self::new()
  }
}

impl fmt::Display for TransferId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

// ============================================================================
// Enum Types
// ============================================================================

/// User online status.
#[derive(
  Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode,
)]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
  /// User is online and active
  #[default]
  Online,
  /// User is offline
  Offline,
  /// User is busy and should not be disturbed
  Busy,
  /// User is away (idle for 5+ minutes)
  Away,
}

impl fmt::Display for UserStatus {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Online => write!(f, "Online"),
      Self::Offline => write!(f, "Offline"),
      Self::Busy => write!(f, "Busy"),
      Self::Away => write!(f, "Away"),
    }
  }
}

/// Room type.
#[derive(
  Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode,
)]
#[serde(rename_all = "snake_case")]
pub enum RoomType {
  /// Standard chat room for text/media messaging
  #[default]
  Chat,
  /// Theater room for synchronized video watching with danmaku
  Theater,
}

impl fmt::Display for RoomType {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Chat => write!(f, "Chat"),
      Self::Theater => write!(f, "Theater"),
    }
  }
}

/// Media type for calls and screen sharing.
#[derive(
  Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode,
)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
  /// Audio only (voice call)
  #[default]
  Audio,
  /// Audio and video (video call)
  Video,
  /// Screen sharing
  ScreenShare,
}

impl fmt::Display for MediaType {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Audio => write!(f, "Audio"),
      Self::Video => write!(f, "Video"),
      Self::ScreenShare => write!(f, "Screen Share"),
    }
  }
}

/// Danmaku (bullet comment) display position.
#[derive(
  Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode,
)]
#[serde(rename_all = "snake_case")]
pub enum DanmakuPosition {
  /// Scrolling from right to left (default)
  #[default]
  Scroll,
  /// Fixed at top
  Top,
  /// Fixed at bottom
  Bottom,
}

/// Message content type.
#[derive(
  Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode,
)]
#[serde(rename_all = "snake_case")]
pub enum MessageContentType {
  /// Plain text message
  #[default]
  Text,
  /// Sticker message
  Sticker,
  /// Voice message (Opus encoded)
  Voice,
  /// Image message
  Image,
  /// File message
  File,
  /// System message (join, leave, etc.)
  System,
}

impl fmt::Display for MessageContentType {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Text => write!(f, "Text"),
      Self::Sticker => write!(f, "Sticker"),
      Self::Voice => write!(f, "Voice"),
      Self::Image => write!(f, "Image"),
      Self::File => write!(f, "File"),
      Self::System => write!(f, "System"),
    }
  }
}

/// Reaction action type.
#[derive(
  Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode,
)]
#[serde(rename_all = "snake_case")]
pub enum ReactionAction {
  /// Add a reaction
  #[default]
  Add,
  /// Remove a reaction
  Remove,
}

/// Mute information for room members.
///
/// Uses timestamp-based encoding for bitcode serialization.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MuteInfo {
  /// Not muted
  #[default]
  NotMuted,
  /// Muted for a specific duration
  Timed {
    /// When the mute expires (Unix timestamp in nanoseconds)
    expires_at_nanos: i64,
  },
  /// Permanently muted
  Permanent,
}

impl MuteInfo {
  /// Create a not muted state.
  #[must_use]
  pub const fn not_muted() -> Self {
    Self::NotMuted
  }

  /// Create a permanent mute.
  #[must_use]
  pub const fn permanent() -> Self {
    Self::Permanent
  }

  /// Create a timed mute for the specified duration.
  #[must_use]
  pub fn timed(duration: Duration) -> Self {
    let expires_at = Utc::now() + duration;
    Self::Timed {
      expires_at_nanos: expires_at.timestamp_nanos_opt().unwrap_or(0),
    }
  }

  /// Create a timed mute from a specific expiration time.
  #[must_use]
  pub fn timed_at(expires_at: DateTime<Utc>) -> Self {
    Self::Timed {
      expires_at_nanos: expires_at.timestamp_nanos_opt().unwrap_or(0),
    }
  }

  /// Check if the user is currently muted.
  #[must_use]
  pub fn is_muted(&self) -> bool {
    match self {
      Self::NotMuted => false,
      Self::Timed { expires_at_nanos } => {
        let now_nanos = Utc::now().timestamp_nanos_opt().unwrap_or(i64::MAX);
        now_nanos < *expires_at_nanos
      }
      Self::Permanent => true,
    }
  }

  /// Get the expiration time if this is a timed mute.
  #[must_use]
  pub fn expires_at(&self) -> Option<DateTime<Utc>> {
    match self {
      Self::Timed { expires_at_nanos } => Some(Utc.timestamp_nanos(*expires_at_nanos)),
      _ => None,
    }
  }
}

/// Room member role.
#[derive(
  Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode,
)]
#[serde(rename_all = "snake_case")]
pub enum RoomRole {
  /// Room owner with full control
  Owner,
  /// Room admin with moderation powers
  Admin,
  /// Regular room member
  #[default]
  Member,
}

impl fmt::Display for RoomRole {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Owner => write!(f, "Owner"),
      Self::Admin => write!(f, "Admin"),
      Self::Member => write!(f, "Member"),
    }
  }
}

impl PartialOrd for RoomRole {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for RoomRole {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    let rank = |role: &Self| match role {
      Self::Owner => 2,
      Self::Admin => 1,
      Self::Member => 0,
    };
    rank(self).cmp(&rank(other))
  }
}

// ============================================================================
// Struct Types
// ============================================================================

/// User information.
///
/// Uses timestamp-based encoding for `DateTime` fields to work with bitcode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct UserInfo {
  /// User ID
  pub user_id: UserId,
  /// Username (unique identifier for login)
  pub username: String,
  /// Display nickname
  pub nickname: String,
  /// User status
  pub status: UserStatus,
  /// Avatar URL (optional, generated identicon if not set)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub avatar_url: Option<String>,
  /// User's self-introduction/bio
  #[serde(default, skip_serializing_if = "String::is_empty")]
  pub bio: String,
  /// Account creation timestamp (Unix timestamp in nanoseconds)
  pub created_at_nanos: i64,
  /// Last activity timestamp (Unix timestamp in nanoseconds)
  pub last_seen_nanos: i64,
}

impl UserInfo {
  /// Create a new user info.
  #[must_use]
  pub fn new(user_id: UserId, username: String, nickname: String) -> Self {
    let now_nanos = Utc::now().timestamp_nanos_opt().unwrap_or(0);
    Self {
      user_id,
      username,
      nickname,
      status: UserStatus::Online,
      avatar_url: None,
      bio: String::new(),
      created_at_nanos: now_nanos,
      last_seen_nanos: now_nanos,
    }
  }

  /// Get the account creation timestamp.
  #[must_use]
  pub fn created_at(&self) -> DateTime<Utc> {
    Utc.timestamp_nanos(self.created_at_nanos)
  }

  /// Get the last activity timestamp.
  #[must_use]
  pub fn last_seen(&self) -> DateTime<Utc> {
    Utc.timestamp_nanos(self.last_seen_nanos)
  }

  /// Set the last activity timestamp.
  pub fn touch(&mut self) {
    self.last_seen_nanos = Utc::now().timestamp_nanos_opt().unwrap_or(0);
  }
}

/// Room information.
///
/// Uses timestamp-based encoding for `DateTime` fields to work with bitcode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct RoomInfo {
  /// Room ID
  pub room_id: RoomId,
  /// Room name
  pub name: String,
  /// Room description
  #[serde(default, skip_serializing_if = "String::is_empty")]
  pub description: String,
  /// Room type (Chat or Theater)
  pub room_type: RoomType,
  /// Room owner's user ID
  pub owner_id: UserId,
  /// Room password hash (if password protected)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub password_hash: Option<String>,
  /// Maximum number of members (default 8)
  pub max_members: u8,
  /// Current member count
  pub member_count: u8,
  /// Room creation timestamp (Unix timestamp in nanoseconds)
  pub created_at_nanos: i64,
  /// Room announcement
  #[serde(default, skip_serializing_if = "String::is_empty")]
  pub announcement: String,
  /// Theater video URL (only for Theater rooms)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub video_url: Option<String>,
}

impl RoomInfo {
  /// Create a new room info.
  #[must_use]
  pub fn new(room_id: RoomId, name: String, room_type: RoomType, owner_id: UserId) -> Self {
    Self {
      room_id,
      name,
      description: String::new(),
      room_type,
      owner_id,
      password_hash: None,
      max_members: 8,
      member_count: 1,
      created_at_nanos: Utc::now().timestamp_nanos_opt().unwrap_or(0),
      announcement: String::new(),
      video_url: None,
    }
  }

  /// Get the room creation timestamp.
  #[must_use]
  pub fn created_at(&self) -> DateTime<Utc> {
    Utc.timestamp_nanos(self.created_at_nanos)
  }

  /// Check if the room is password protected.
  #[must_use]
  pub const fn is_password_protected(&self) -> bool {
    self.password_hash.is_some()
  }

  /// Check if the room is full.
  #[must_use]
  pub fn is_full(&self) -> bool {
    self.member_count >= self.max_members
  }
}

/// Room member information.
///
/// Uses timestamp-based encoding for `DateTime` fields to work with bitcode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct MemberInfo {
  /// User ID
  pub user_id: UserId,
  /// Display nickname in this room
  pub nickname: String,
  /// Member role
  pub role: RoomRole,
  /// Mute status
  pub mute_info: MuteInfo,
  /// Join timestamp (Unix timestamp in nanoseconds)
  pub joined_at_nanos: i64,
  /// Last activity in room (Unix timestamp in nanoseconds)
  pub last_active_nanos: i64,
}

impl MemberInfo {
  /// Create a new member info.
  #[must_use]
  pub fn new(user_id: UserId, nickname: String, role: RoomRole) -> Self {
    let now_nanos = Utc::now().timestamp_nanos_opt().unwrap_or(0);
    Self {
      user_id,
      nickname,
      role,
      mute_info: MuteInfo::NotMuted,
      joined_at_nanos: now_nanos,
      last_active_nanos: now_nanos,
    }
  }

  /// Get the join timestamp.
  #[must_use]
  pub fn joined_at(&self) -> DateTime<Utc> {
    Utc.timestamp_nanos(self.joined_at_nanos)
  }

  /// Get the last activity timestamp.
  #[must_use]
  pub fn last_active(&self) -> DateTime<Utc> {
    Utc.timestamp_nanos(self.last_active_nanos)
  }

  /// Check if the member is currently muted.
  #[must_use]
  pub fn is_muted(&self) -> bool {
    self.mute_info.is_muted()
  }

  /// Update the last activity timestamp.
  pub fn touch(&mut self) {
    self.last_active_nanos = Utc::now().timestamp_nanos_opt().unwrap_or(0);
  }
}

/// Image metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct ImageMeta {
  /// Image width in pixels
  pub width: u32,
  /// Image height in pixels
  pub height: u32,
  /// File size in bytes
  pub size: u64,
  /// MIME type (e.g., "image/jpeg", "image/png")
  pub mime_type: String,
  /// Thumbnail URL (optional)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub thumbnail_url: Option<String>,
  /// Original image URL
  pub original_url: String,
}

impl ImageMeta {
  /// Create a new image metadata.
  #[must_use]
  pub fn new(width: u32, height: u32, size: u64, mime_type: String, original_url: String) -> Self {
    Self {
      width,
      height,
      size,
      mime_type,
      thumbnail_url: None,
      original_url,
    }
  }

  /// Calculate aspect ratio.
  #[must_use]
  pub fn aspect_ratio(&self) -> f64 {
    if self.height == 0 {
      return 0.0;
    }
    f64::from(self.width) / f64::from(self.height)
  }
}

/// Subtitle entry for theater mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct SubtitleEntry {
  /// Start time in milliseconds
  pub start_ms: u64,
  /// End time in milliseconds
  pub end_ms: u64,
  /// Subtitle text content
  pub text: String,
}

impl SubtitleEntry {
  /// Create a new subtitle entry.
  #[must_use]
  pub fn new(start_ms: u64, end_ms: u64, text: String) -> Self {
    Self {
      start_ms,
      end_ms,
      text,
    }
  }

  /// Check if the subtitle should be displayed at the given time.
  #[must_use]
  pub const fn is_active_at(&self, time_ms: u64) -> bool {
    time_ms >= self.start_ms && time_ms <= self.end_ms
  }

  /// Duration in milliseconds.
  #[must_use]
  pub const fn duration_ms(&self) -> u64 {
    self.end_ms.saturating_sub(self.start_ms)
  }
}

/// Network quality level.
#[derive(
  Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode,
)]
#[serde(rename_all = "snake_case")]
pub enum NetworkQuality {
  /// Excellent quality (< 100ms RTT, < 1% packet loss)
  Excellent,
  /// Good quality (100-200ms RTT, 1-3% packet loss)
  #[default]
  Good,
  /// Fair quality (200-400ms RTT, 3-10% packet loss)
  Fair,
  /// Poor quality (> 400ms RTT, > 10% packet loss)
  Poor,
}

impl fmt::Display for NetworkQuality {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Excellent => write!(f, "Excellent"),
      Self::Good => write!(f, "Good"),
      Self::Fair => write!(f, "Fair"),
      Self::Poor => write!(f, "Poor"),
    }
  }
}

impl NetworkQuality {
  /// Classify network quality based on RTT and packet loss.
  #[must_use]
  pub fn from_metrics(rtt_ms: u64, packet_loss_percent: f64) -> Self {
    match (rtt_ms, packet_loss_percent) {
      (rtt, loss) if rtt < 100 && loss < 1.0 => Self::Excellent,
      (rtt, loss) if rtt < 200 && loss < 3.0 => Self::Good,
      (rtt, loss) if rtt < 400 && loss < 10.0 => Self::Fair,
      _ => Self::Poor,
    }
  }

  /// Get the recommended video quality for this network level.
  #[must_use]
  pub const fn recommended_video_quality(&self) -> &str {
    match self {
      Self::Excellent => "1080p",
      Self::Good => "720p",
      Self::Fair => "480p",
      Self::Poor => "360p",
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

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
    let id1 = MessageId::new_v4();
    let id2 = MessageId::new_v4();
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
    let past_time = Utc::now() - Duration::hours(1);
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

  fn test_bitcode_roundtrip<T: Encode + for<'a> Decode<'a> + PartialEq + std::fmt::Debug>(
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
}
