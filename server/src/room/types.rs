//! Room-related types and error definitions.

use message::types::{MemberInfo, RoomId, RoomInfo, RoomRole, UserId};

// =============================================================================
// Error Types
// =============================================================================

/// Error types for room operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomError {
  /// Room not found.
  RoomNotFound,
  /// User is not a member of the room.
  NotMember,
  /// User is already a member.
  AlreadyMember,
  /// User is banned from the room.
  UserBanned,
  /// Room is full.
  RoomFull,
  /// Invalid password.
  InvalidPassword(String),
  /// Insufficient permission.
  InsufficientPermission,
  /// Invalid input.
  InvalidInput(String),
  /// User already in another room.
  UserAlreadyInRoom,
  /// User not in any room.
  UserNotInRoom,
  /// Cannot promote owner.
  CannotPromoteOwner,
  /// Cannot demote owner.
  CannotDemoteOwner,
  /// Target is not an admin.
  NotAdmin,
  /// User already owns a room of the same type.
  AlreadyOwnerOfSameType,
  /// User is not banned.
  NotBanned,
  /// Invalid room name (contains forbidden characters).
  InvalidRoomName(String),
}

impl std::fmt::Display for RoomError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::RoomNotFound => write!(f, "Room not found"),
      Self::NotMember => write!(f, "User is not a member of the room"),
      Self::AlreadyMember => write!(f, "User is already a member"),
      Self::UserBanned => write!(f, "User is banned from the room"),
      Self::RoomFull => write!(f, "Room is full"),
      Self::InvalidPassword(msg) => write!(f, "Invalid password: {}", msg),
      Self::InsufficientPermission => write!(f, "Insufficient permission"),
      Self::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
      Self::UserAlreadyInRoom => write!(f, "User is already in a room"),
      Self::UserNotInRoom => write!(f, "User is not in any room"),
      Self::CannotPromoteOwner => write!(f, "Cannot promote owner"),
      Self::CannotDemoteOwner => write!(f, "Cannot demote owner"),
      Self::NotAdmin => write!(f, "Target is not an admin"),
      Self::AlreadyOwnerOfSameType => write!(f, "User already owns a room of the same type"),
      Self::NotBanned => write!(f, "User is not banned"),
      Self::InvalidRoomName(msg) => write!(f, "Invalid room name: {}", msg),
    }
  }
}

impl std::error::Error for RoomError {}

// =============================================================================
// Helper Types
// =============================================================================

/// Result of leaving a room.
#[derive(Debug, Clone)]
pub struct LeaveRoomResult {
  /// Room ID.
  pub room_id: RoomId,
  /// Room info (None if room was destroyed).
  pub room_info: Option<RoomInfo>,
  /// Remaining members (empty if room was destroyed).
  pub members: Vec<MemberInfo>,
  /// New owner if ownership was transferred.
  pub ownership_transfer: Option<UserId>,
  /// Whether the room was destroyed.
  pub room_destroyed: bool,
  /// The removed member info.
  pub removed_member: MemberInfo,
}

/// Result of permission check.
#[derive(Debug, Clone, Copy)]
pub struct PermissionCheckResult {
  /// Whether the action is allowed.
  pub can_act: bool,
  /// Actor's role.
  pub actor_role: RoomRole,
  /// Target's role.
  pub target_role: RoomRole,
}
