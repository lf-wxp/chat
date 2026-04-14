//! Room moderation, theater, and profile signaling messages.

use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::types::{RoomId, UserId};

/// Mute a member in a room.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct MuteMember {
  /// Room ID.
  pub room_id: RoomId,
  /// Target user ID.
  pub target: UserId,
  /// Mute duration in seconds (None = permanent).
  pub duration_secs: Option<u64>,
}

/// Unmute a member in a room.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct UnmuteMember {
  /// Room ID.
  pub room_id: RoomId,
  /// Target user ID.
  pub target: UserId,
}

/// Ban a member from a room (kicked + cannot rejoin).
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct BanMember {
  /// Room ID.
  pub room_id: RoomId,
  /// Target user ID.
  pub target: UserId,
}

/// Unban a member from a room.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct UnbanMember {
  /// Room ID.
  pub room_id: RoomId,
  /// Target user ID.
  pub target: UserId,
}

/// Promote a member to Admin role.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct PromoteAdmin {
  /// Room ID.
  pub room_id: RoomId,
  /// Target user ID.
  pub target: UserId,
}

/// Demote an Admin back to Member role.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct DemoteAdmin {
  /// Room ID.
  pub room_id: RoomId,
  /// Target user ID.
  pub target: UserId,
}

/// User nickname change broadcast.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct NicknameChange {
  /// User ID.
  pub user_id: UserId,
  /// New nickname.
  pub new_nickname: String,
}

/// Room announcement update broadcast.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct RoomAnnouncement {
  /// Room ID.
  pub room_id: RoomId,
  /// Announcement content.
  pub content: String,
}

/// Moderation action type for notifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModerationAction {
  /// User was kicked from room.
  Kicked,
  /// User was muted.
  Muted,
  /// User was unmuted.
  Unmuted,
  /// User was banned.
  Banned,
  /// User was unbanned.
  Unbanned,
  /// User was promoted to admin.
  Promoted,
  /// User was demoted from admin.
  Demoted,
}

/// Notification of moderation action to room members.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct ModerationNotification {
  /// Room ID.
  pub room_id: RoomId,
  /// Moderation action type.
  pub action: ModerationAction,
  /// Target user ID.
  pub target: UserId,
  /// Optional reason for the action.
  pub reason: Option<String>,
  /// Duration in seconds (for mute).
  pub duration_secs: Option<u64>,
}

// ---------------------------------------------------------------------------
// Theater Signaling Messages
// ---------------------------------------------------------------------------

/// Mute all viewers in theater room.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct TheaterMuteAll {
  /// Room ID.
  pub room_id: RoomId,
}

/// Transfer theater ownership.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct TheaterTransferOwner {
  /// Room ID.
  pub room_id: RoomId,
  /// Target user ID to transfer ownership to.
  pub target: UserId,
}
