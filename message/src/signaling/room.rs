//! Room management signaling messages.

use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::types::{MemberInfo, RoomId, RoomInfo, RoomType, UserId};

/// Create room request.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct CreateRoom {
  /// Room name.
  pub name: String,
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
