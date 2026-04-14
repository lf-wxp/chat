//! Struct types for the WebRTC Chat Application.
//!
//! This module defines the main struct types including `UserInfo`, `RoomInfo`,
//! `MemberInfo`, and `ImageMeta`.

use bitcode::{Decode, Encode};
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use super::enums::{RoomType, UserStatus};
use super::identifiers::{RoomId, UserId};
use super::mute::MuteInfo;
use super::role::RoomRole;

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
