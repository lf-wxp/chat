//! WebRTC signaling protocol definitions
//!
//! All signaling messages transmitted over WebSocket are defined here.
//! WebSocket is used solely for signaling — it does not carry chat messages or file data.

use serde::{Deserialize, Serialize};

use crate::types::Id;

/// Signaling message — transmitted over WebSocket
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SignalMessage {
  // ========================================================================
  // Authentication
  // ========================================================================
  /// Registration request
  Register { username: String, password: String },
  /// Login request
  Login { username: String, password: String },
  /// Authentication success response
  AuthSuccess {
    user_id: Id,
    token: String,
    username: String,
  },
  /// Authentication failure response
  AuthError { reason: String },
  /// Token authentication (first message after WebSocket connection)
  TokenAuth { token: String },
  /// ICE server configuration (sent by server after successful authentication)
  IceConfig { ice_servers: Vec<String> },

  // ========================================================================
  // WebRTC Signaling
  // ========================================================================
  /// SDP Offer
  SdpOffer { from: Id, to: Id, sdp: String },
  /// SDP Answer
  SdpAnswer { from: Id, to: Id, sdp: String },
  /// ICE Candidate
  IceCandidate { from: Id, to: Id, candidate: String },

  // ========================================================================
  // User Status
  // ========================================================================
  /// Online user list update
  UserListUpdate { users: Vec<OnlineUser> },
  /// User status change
  UserStatusChange { user_id: Id, status: UserStatus },

  // ========================================================================
  // Connection Invite
  // ========================================================================
  /// Send connection invite
  ConnectionInvite {
    from: Id,
    to: Id,
    /// Optional message
    message: Option<String>,
    /// Invite type
    invite_type: InviteType,
  },
  /// Invite response
  InviteResponse { from: Id, to: Id, accepted: bool },
  /// Invite timeout notification
  InviteTimeout { from: Id, to: Id },

  // ========================================================================
  // Room Management
  // ========================================================================
  /// Create room
  CreateRoom {
    name: String,
    description: Option<String>,
    password: Option<String>,
    max_members: u32,
    room_type: RoomType,
  },
  /// Room created success response
  RoomCreated { room_id: Id },
  /// Join room
  JoinRoom {
    room_id: Id,
    password: Option<String>,
  },
  /// Leave room
  LeaveRoom { room_id: Id },
  /// Room member change notification
  RoomMemberUpdate {
    room_id: Id,
    members: Vec<RoomMemberInfo>,
  },
  /// Room list update
  RoomListUpdate { rooms: Vec<RoomInfo> },
  /// Room error
  RoomError { reason: String },

  // ========================================================================
  // Owner Management Operations
  // ========================================================================
  /// Kick member
  KickMember { room_id: Id, target_user_id: Id },
  /// Mute / unmute member
  MuteMember {
    room_id: Id,
    target_user_id: Id,
    muted: bool,
  },
  /// Mute all members
  MuteAll { room_id: Id, muted: bool },
  /// Transfer ownership
  TransferOwner { room_id: Id, new_owner_id: Id },
  /// Kicked notification
  Kicked { room_id: Id, reason: Option<String> },
  /// Mute status change notification
  MuteStatusChanged { room_id: Id, muted: bool },

  // ========================================================================
  // Theater Control
  // ========================================================================
  /// Playback control (owner only)
  TheaterControl { room_id: Id, action: TheaterAction },
  /// Playback progress sync
  TheaterSync {
    room_id: Id,
    current_time: f64,
    is_playing: bool,
  },

  // ========================================================================
  // Call Control
  // ========================================================================
  /// Initiate call invite
  CallInvite {
    from: Id,
    to: Vec<Id>,
    media_type: crate::types::MediaType,
  },
  /// Call response
  CallResponse { from: Id, to: Id, accepted: bool },
  /// Hang up call
  CallHangup { from: Id, room_id: Option<Id> },
  /// Media track change notification
  MediaTrackChanged {
    from: Id,
    video_enabled: bool,
    audio_enabled: bool,
  },

  // ========================================================================
  // Invite Link
  // ========================================================================
  /// Create invite link request
  CreateInviteLink {
    /// Invite type
    invite_type: InviteType,
    /// Target room ID (required for Room-type invites)
    room_id: Option<Id>,
  },
  /// Invite link created success response
  InviteLinkCreated {
    /// Invite code (short code for composing the link)
    code: String,
    /// Expiration timestamp (milliseconds)
    expires_at: i64,
    /// Invite type
    invite_type: InviteType,
  },
  /// Join via invite link
  JoinByInviteLink {
    /// Invite code
    code: String,
  },
  /// Invite link error
  InviteLinkError { reason: String },

  // ========================================================================
  // Heartbeat
  // ========================================================================
  /// Heartbeat Ping
  Ping,
  /// Heartbeat Pong
  Pong,

  // ========================================================================
  // Error
  // ========================================================================
  /// Generic error
  Error { code: u32, message: String },
}

/// Online user info
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnlineUser {
  pub user_id: Id,
  pub username: String,
  pub status: UserStatus,
  pub avatar: Option<String>,
}

/// User online status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum UserStatus {
  #[default]
  Online,
  Offline,
  Busy,
  Away,
}

/// Invite type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InviteType {
  /// One-on-one chat
  Chat,
  /// Audio call
  AudioCall,
  /// Video call
  VideoCall,
  /// Join room
  Room,
}

/// Room type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoomType {
  /// Regular chat room
  Chat,
  /// Theater
  Theater,
}

/// Room member info
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoomMemberInfo {
  pub user_id: Id,
  pub role: MemberRole,
  pub muted: bool,
}

/// Member role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemberRole {
  /// Owner
  Owner,
  /// Regular member
  Member,
  /// Viewer (theater)
  Viewer,
}

/// Room info (for list display)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoomInfo {
  pub room_id: Id,
  pub name: String,
  pub description: Option<String>,
  pub room_type: RoomType,
  pub member_count: u32,
  pub max_members: u32,
  pub has_password: bool,
  pub owner_name: String,
  /// Theater-specific: whether currently playing
  pub is_playing: Option<bool>,
}

/// Theater control action
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TheaterAction {
  /// Play
  Play,
  /// Pause
  Pause,
  /// Seek to specified time (seconds)
  Seek(f64),
  /// Change video source
  ChangeSource {
    source_type: VideoSourceType,
    url: Option<String>,
  },
}

/// Video source type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoSourceType {
  /// Local file
  Local,
  /// Online URL
  Online,
}
