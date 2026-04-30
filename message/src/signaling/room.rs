//! Room management signaling messages.

use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::types::{MemberInfo, RoomId, RoomInfo, RoomType, UserId};

/// Create room request.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct CreateRoom {
  /// Room name.
  pub name: String,
  /// Optional room description shown in the room list. Empty string
  /// means no description.
  #[serde(default, skip_serializing_if = "String::is_empty")]
  pub description: String,
  /// Room type (Chat or Theater).
  pub room_type: RoomType,
  /// Optional password for the room.
  pub password: Option<String>,
  /// Maximum number of participants (default: 8).
  pub max_participants: u8,
}

/// Join room request.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct JoinRoom {
  /// Room ID to join.
  pub room_id: RoomId,
  /// Optional password if the room is password-protected.
  pub password: Option<String>,
}

/// Leave room request.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct LeaveRoom {
  /// Room ID to leave.
  pub room_id: RoomId,
}

/// Room list update.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct RoomListUpdate {
  /// List of rooms.
  pub rooms: Vec<RoomInfo>,
}

/// Room member list update.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct RoomMemberUpdate {
  /// Room ID.
  pub room_id: RoomId,
  /// List of room members.
  pub members: Vec<MemberInfo>,
}

/// Kick member from room.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct KickMember {
  /// Room ID.
  pub room_id: RoomId,
  /// Target user ID to kick.
  pub target: UserId,
}

/// Transfer room ownership.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct TransferOwnership {
  /// Room ID.
  pub room_id: RoomId,
  /// Target user ID to transfer ownership to.
  pub target: UserId,
}

/// Update room name and description (Owner only — Req 4.5).
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct UpdateRoomInfo {
  /// Room ID being updated.
  pub room_id: RoomId,
  /// New room name (subject to room-name validation rules).
  pub name: String,
  /// New room description (may be empty).
  #[serde(default, skip_serializing_if = "String::is_empty")]
  pub description: String,
}

/// Update or clear the room password (Owner only — Req 4.5a / 4.5b).
///
/// `None` clears the password (room becomes public);
/// `Some(non_empty)` sets a new password.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct UpdateRoomPassword {
  /// Room ID being updated.
  pub room_id: RoomId,
  /// New password (None = clear).
  pub password: Option<String>,
}

/// Invite a specific user into a room (Req 4.3).
///
/// The server forwards this message to the target user as a
/// `RoomInvite` so the target can show an "incoming room invite"
/// modal and accept / decline.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct RoomInvite {
  /// Room being shared.
  pub room_id: RoomId,
  /// Inviting user (filled in by the server when forwarding).
  pub from: UserId,
  /// Target user the invite is destined for.
  pub to: UserId,
  /// Optional human-readable note from the inviter.
  #[serde(default, skip_serializing_if = "String::is_empty")]
  pub note: String,
}

/// Response from the invitee to a [`RoomInvite`] (Req 4.4).
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct RoomInviteResponse {
  /// Room the invite referred to.
  pub room_id: RoomId,
  /// User who originally sent the invite (target of this response).
  pub to: UserId,
  /// Whether the invite was accepted.
  pub accepted: bool,
}

// ---------------------------------------------------------------------------
// Room Response Messages
// ---------------------------------------------------------------------------

/// Room created response.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct RoomCreated {
  /// Created room ID.
  pub room_id: RoomId,
  /// Room info.
  pub room_info: RoomInfo,
}

/// Room joined response.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct RoomJoined {
  /// Joined room ID.
  pub room_id: RoomId,
  /// Room info.
  pub room_info: RoomInfo,
  /// Current members in the room.
  pub members: Vec<MemberInfo>,
}

/// Room left response.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct RoomLeft {
  /// Left room ID.
  pub room_id: RoomId,
  /// Whether the room was destroyed (last member left).
  pub room_destroyed: bool,
}

/// Owner changed notification.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct OwnerChanged {
  /// Room ID.
  pub room_id: RoomId,
  /// Old owner user ID.
  pub old_owner: UserId,
  /// New owner user ID.
  pub new_owner: UserId,
}

/// Mute status change notification.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct MuteStatusChange {
  /// Room ID.
  pub room_id: RoomId,
  /// Target user ID.
  pub target: UserId,
  /// Mute info.
  pub mute_info: crate::types::MuteInfo,
}
