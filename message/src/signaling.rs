//! Signaling message types for WebSocket communication.
//!
//! This module defines all signaling message types exchanged between client and server
//! over WebSocket. All messages use bitcode binary serialization.

use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::types::{
  MediaType, MemberInfo, RoomId, RoomInfo, RoomType, UserId, UserInfo, UserStatus,
};

// =============================================================================
// Message Type Discriminator Constants
// =============================================================================

/// Signaling message type discriminator values.
///
/// These values are used as the first byte after the magic number (0xBCBC)
/// to identify the message type during deserialization.
pub mod discriminator {
  // Connection & Authentication (0x00-0x06)
  /// JWT authentication message type.
  pub const TOKEN_AUTH: u8 = 0x00;
  /// Authentication success response message type.
  pub const AUTH_SUCCESS: u8 = 0x01;
  /// Authentication failure response message type.
  pub const AUTH_FAILURE: u8 = 0x02;
  /// User logout message type.
  pub const USER_LOGOUT: u8 = 0x03;
  /// Heartbeat ping message type.
  pub const PING: u8 = 0x04;
  /// Heartbeat pong message type.
  pub const PONG: u8 = 0x05;
  /// Error response message type.
  pub const ERROR_RESPONSE: u8 = 0x06;
  /// Session invalidated by another device login.
  pub const SESSION_INVALIDATED: u8 = 0x07;

  // User Discovery & Status (0x10-0x11)
  /// User list update message type.
  pub const USER_LIST_UPDATE: u8 = 0x10;
  /// User status change message type.
  pub const USER_STATUS_CHANGE: u8 = 0x11;

  // Connection Invitation (0x20-0x24)
  /// Connection invitation message type.
  pub const CONNECTION_INVITE: u8 = 0x20;
  /// Invitation accepted message type.
  pub const INVITE_ACCEPTED: u8 = 0x21;
  /// Invitation declined message type.
  pub const INVITE_DECLINED: u8 = 0x22;
  /// Invitation timeout message type.
  pub const INVITE_TIMEOUT: u8 = 0x23;
  /// Multi-user invitation message type.
  pub const MULTI_INVITE: u8 = 0x24;

  // SDP / ICE Signaling (0x30-0x32)
  /// SDP offer message type.
  pub const SDP_OFFER: u8 = 0x30;
  /// SDP answer message type.
  pub const SDP_ANSWER: u8 = 0x31;
  /// ICE candidate message type.
  pub const ICE_CANDIDATE: u8 = 0x32;

  // Peer Tracking (0x40-0x42)
  /// Peer connection established message type.
  pub const PEER_ESTABLISHED: u8 = 0x40;
  /// Peer connection closed message type.
  pub const PEER_CLOSED: u8 = 0x41;
  /// Active peers list message type.
  pub const ACTIVE_PEERS_LIST: u8 = 0x42;

  // Room Management (0x50-0x56)
  /// Create room message type.
  pub const CREATE_ROOM: u8 = 0x50;
  /// Join room message type.
  pub const JOIN_ROOM: u8 = 0x51;
  /// Leave room message type.
  pub const LEAVE_ROOM: u8 = 0x52;
  /// Room list update message type.
  pub const ROOM_LIST_UPDATE: u8 = 0x53;
  /// Room member update message type.
  pub const ROOM_MEMBER_UPDATE: u8 = 0x54;
  /// Kick member message type.
  pub const KICK_MEMBER: u8 = 0x55;
  /// Transfer ownership message type.
  pub const TRANSFER_OWNERSHIP: u8 = 0x56;

  // Call Signaling (0x60-0x63)
  /// Call invitation message type.
  pub const CALL_INVITE: u8 = 0x60;
  /// Call accept message type.
  pub const CALL_ACCEPT: u8 = 0x61;
  /// Call decline message type.
  pub const CALL_DECLINE: u8 = 0x62;
  /// Call end message type.
  pub const CALL_END: u8 = 0x63;

  // Theater Signaling (0x70-0x71)
  /// Theater mute all message type.
  pub const THEATER_MUTE_ALL: u8 = 0x70;
  /// Theater transfer owner message type.
  pub const THEATER_TRANSFER_OWNER: u8 = 0x71;

  // Room Moderation & Profile (0x75-0x7D)
  /// Mute member message type.
  pub const MUTE_MEMBER: u8 = 0x75;
  /// Unmute member message type.
  pub const UNMUTE_MEMBER: u8 = 0x76;
  /// Ban member message type.
  pub const BAN_MEMBER: u8 = 0x77;
  /// Unban member message type.
  pub const UNBAN_MEMBER: u8 = 0x78;
  /// Promote admin message type.
  pub const PROMOTE_ADMIN: u8 = 0x79;
  /// Demote admin message type.
  pub const DEMOTE_ADMIN: u8 = 0x7A;
  /// Nickname change message type.
  pub const NICKNAME_CHANGE: u8 = 0x7B;
  /// Room announcement message type.
  pub const ROOM_ANNOUNCEMENT: u8 = 0x7C;
  /// Moderation notification message type.
  pub const MODERATION_NOTIFICATION: u8 = 0x7D;
}

// =============================================================================
// Connection & Authentication Messages
// =============================================================================

/// JWT authentication on WebSocket connect / page refresh.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct TokenAuth {
  /// JWT token for authentication.
  pub token: String,
}

/// Authentication success response.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct AuthSuccess {
  /// Authenticated user ID.
  pub user_id: UserId,
  /// Authenticated username.
  pub username: String,
}

/// Authentication failure response.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct AuthFailure {
  /// Failure reason.
  pub reason: String,
}

/// Active logout notification.
#[derive(Debug, Clone, Default, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct UserLogout {}

/// Heartbeat ping.
#[derive(Debug, Clone, Default, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct Ping {}

/// Heartbeat pong.
#[derive(Debug, Clone, Default, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct Pong {}

/// Session invalidated by another device login.
/// Sent to old connection when user logs in from a new device.
#[derive(Debug, Clone, Default, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct SessionInvalidated {}

// =============================================================================
// User Discovery & Status Messages
// =============================================================================

/// Full/incremental online user list update.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct UserListUpdate {
  /// List of online users.
  pub users: Vec<UserInfo>,
}

/// User status/signature change broadcast.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct UserStatusChange {
  /// User ID.
  pub user_id: UserId,
  /// New status.
  pub status: UserStatus,
  /// Optional signature/message.
  pub signature: Option<String>,
}

// =============================================================================
// Connection Invitation Messages
// =============================================================================

/// Connection invitation.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct ConnectionInvite {
  /// Inviter user ID.
  pub from: UserId,
  /// Target user ID.
  pub to: UserId,
  /// Optional invitation note.
  pub note: Option<String>,
}

/// Invitation accepted.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct InviteAccepted {
  /// Inviter user ID.
  pub from: UserId,
  /// Invitee user ID.
  pub to: UserId,
}

/// Invitation declined.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct InviteDeclined {
  /// Inviter user ID.
  pub from: UserId,
  /// Invitee user ID.
  pub to: UserId,
}

/// Invitation timed out.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct InviteTimeout {
  /// Inviter user ID.
  pub from: UserId,
  /// Invitee user ID.
  pub to: UserId,
}

/// Multi-user invitation.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct MultiInvite {
  /// Inviter user ID.
  pub from: UserId,
  /// Target user IDs.
  pub targets: Vec<UserId>,
}

// =============================================================================
// SDP / ICE Signaling Messages
// =============================================================================

/// SDP Offer forwarding.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct SdpOffer {
  /// Sender user ID.
  pub from: UserId,
  /// Target user ID.
  pub to: UserId,
  /// SDP offer string.
  pub sdp: String,
}

/// SDP Answer forwarding.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct SdpAnswer {
  /// Sender user ID.
  pub from: UserId,
  /// Target user ID.
  pub to: UserId,
  /// SDP answer string.
  pub sdp: String,
}

/// ICE Candidate forwarding.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct IceCandidate {
  /// Sender user ID.
  pub from: UserId,
  /// Target user ID.
  pub to: UserId,
  /// ICE candidate string.
  pub candidate: String,
}

// =============================================================================
// Peer Tracking Messages
// =============================================================================

/// `PeerConnection` established notification.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct PeerEstablished {
  /// Local user ID.
  pub from: UserId,
  /// Remote user ID.
  pub to: UserId,
}

/// `PeerConnection` closed notification.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct PeerClosed {
  /// Local user ID.
  pub from: UserId,
  /// Remote user ID.
  pub to: UserId,
}

/// Active peers list (for refresh recovery).
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct ActivePeersList {
  /// List of active peer user IDs.
  pub peers: Vec<UserId>,
}

// =============================================================================
// Room Management Messages
// =============================================================================

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

// =============================================================================
// Call Signaling Messages
// =============================================================================

/// Call invitation.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct CallInvite {
  /// Room ID for the call.
  pub room_id: RoomId,
  /// Media type (Audio/Video/ScreenShare).
  pub media_type: MediaType,
}

/// Accept call.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct CallAccept {
  /// Room ID for the call.
  pub room_id: RoomId,
}

/// Decline call.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct CallDecline {
  /// Room ID for the call.
  pub room_id: RoomId,
}

/// End call.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct CallEnd {
  /// Room ID for the call.
  pub room_id: RoomId,
}

// =============================================================================
// Theater Signaling Messages
// =============================================================================

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

// =============================================================================
// Room Moderation & Profile Messages
// =============================================================================

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
  /// Actor user ID (who performed the action).
  pub actor: UserId,
  /// Optional reason for the action.
  pub reason: Option<String>,
}

// =============================================================================
// Unified Signaling Message Enum
// =============================================================================

/// Unified signaling message enum.
///
/// This enum wraps all signaling message types for unified handling.
/// Each variant corresponds to a specific message type discriminator.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SignalingMessage {
  // Connection & Authentication
  /// JWT authentication.
  TokenAuth(TokenAuth),
  /// Authentication success.
  AuthSuccess(AuthSuccess),
  /// Authentication failure.
  AuthFailure(AuthFailure),
  /// User logout.
  UserLogout(UserLogout),
  /// Heartbeat ping.
  Ping(Ping),
  /// Heartbeat pong.
  Pong(Pong),
  /// Error response.
  ErrorResponse(crate::ErrorResponse),
  /// Session invalidated by another device login.
  SessionInvalidated(SessionInvalidated),

  // User Discovery & Status
  /// User list update.
  UserListUpdate(UserListUpdate),
  /// User status change.
  UserStatusChange(UserStatusChange),

  // Connection Invitation
  /// Connection invitation.
  ConnectionInvite(ConnectionInvite),
  /// Invitation accepted.
  InviteAccepted(InviteAccepted),
  /// Invitation declined.
  InviteDeclined(InviteDeclined),
  /// Invitation timed out.
  InviteTimeout(InviteTimeout),
  /// Multi-user invitation.
  MultiInvite(MultiInvite),

  // SDP / ICE Signaling
  /// SDP offer.
  SdpOffer(SdpOffer),
  /// SDP answer.
  SdpAnswer(SdpAnswer),
  /// ICE candidate.
  IceCandidate(IceCandidate),

  // Peer Tracking
  /// Peer connection established.
  PeerEstablished(PeerEstablished),
  /// Peer connection closed.
  PeerClosed(PeerClosed),
  /// Active peers list.
  ActivePeersList(ActivePeersList),

  // Room Management
  /// Create room.
  CreateRoom(CreateRoom),
  /// Join room.
  JoinRoom(JoinRoom),
  /// Leave room.
  LeaveRoom(LeaveRoom),
  /// Room list update.
  RoomListUpdate(RoomListUpdate),
  /// Room member update.
  RoomMemberUpdate(RoomMemberUpdate),
  /// Kick member.
  KickMember(KickMember),
  /// Transfer ownership.
  TransferOwnership(TransferOwnership),

  // Call Signaling
  /// Call invitation.
  CallInvite(CallInvite),
  /// Accept call.
  CallAccept(CallAccept),
  /// Decline call.
  CallDecline(CallDecline),
  /// End call.
  CallEnd(CallEnd),

  // Theater Signaling
  /// Mute all viewers in theater.
  TheaterMuteAll(TheaterMuteAll),
  /// Transfer theater ownership.
  TheaterTransferOwner(TheaterTransferOwner),

  // Room Moderation & Profile
  /// Mute member.
  MuteMember(MuteMember),
  /// Unmute member.
  UnmuteMember(UnmuteMember),
  /// Ban member.
  BanMember(BanMember),
  /// Unban member.
  UnbanMember(UnbanMember),
  /// Promote to admin.
  PromoteAdmin(PromoteAdmin),
  /// Demote from admin.
  DemoteAdmin(DemoteAdmin),
  /// Nickname change.
  NicknameChange(NicknameChange),
  /// Room announcement.
  RoomAnnouncement(RoomAnnouncement),
  /// Moderation notification.
  ModerationNotification(ModerationNotification),
}

impl SignalingMessage {
  /// Returns the message type discriminator for this message.
  #[must_use]
  pub const fn discriminator(&self) -> u8 {
    match self {
      Self::TokenAuth(_) => discriminator::TOKEN_AUTH,
      Self::AuthSuccess(_) => discriminator::AUTH_SUCCESS,
      Self::AuthFailure(_) => discriminator::AUTH_FAILURE,
      Self::UserLogout(_) => discriminator::USER_LOGOUT,
      Self::Ping(_) => discriminator::PING,
      Self::Pong(_) => discriminator::PONG,
      Self::ErrorResponse(_) => discriminator::ERROR_RESPONSE,
      Self::SessionInvalidated(_) => discriminator::SESSION_INVALIDATED,

      Self::UserListUpdate(_) => discriminator::USER_LIST_UPDATE,
      Self::UserStatusChange(_) => discriminator::USER_STATUS_CHANGE,

      Self::ConnectionInvite(_) => discriminator::CONNECTION_INVITE,
      Self::InviteAccepted(_) => discriminator::INVITE_ACCEPTED,
      Self::InviteDeclined(_) => discriminator::INVITE_DECLINED,
      Self::InviteTimeout(_) => discriminator::INVITE_TIMEOUT,
      Self::MultiInvite(_) => discriminator::MULTI_INVITE,

      Self::SdpOffer(_) => discriminator::SDP_OFFER,
      Self::SdpAnswer(_) => discriminator::SDP_ANSWER,
      Self::IceCandidate(_) => discriminator::ICE_CANDIDATE,

      Self::PeerEstablished(_) => discriminator::PEER_ESTABLISHED,
      Self::PeerClosed(_) => discriminator::PEER_CLOSED,
      Self::ActivePeersList(_) => discriminator::ACTIVE_PEERS_LIST,

      Self::CreateRoom(_) => discriminator::CREATE_ROOM,
      Self::JoinRoom(_) => discriminator::JOIN_ROOM,
      Self::LeaveRoom(_) => discriminator::LEAVE_ROOM,
      Self::RoomListUpdate(_) => discriminator::ROOM_LIST_UPDATE,
      Self::RoomMemberUpdate(_) => discriminator::ROOM_MEMBER_UPDATE,
      Self::KickMember(_) => discriminator::KICK_MEMBER,
      Self::TransferOwnership(_) => discriminator::TRANSFER_OWNERSHIP,

      Self::CallInvite(_) => discriminator::CALL_INVITE,
      Self::CallAccept(_) => discriminator::CALL_ACCEPT,
      Self::CallDecline(_) => discriminator::CALL_DECLINE,
      Self::CallEnd(_) => discriminator::CALL_END,

      Self::TheaterMuteAll(_) => discriminator::THEATER_MUTE_ALL,
      Self::TheaterTransferOwner(_) => discriminator::THEATER_TRANSFER_OWNER,

      Self::MuteMember(_) => discriminator::MUTE_MEMBER,
      Self::UnmuteMember(_) => discriminator::UNMUTE_MEMBER,
      Self::BanMember(_) => discriminator::BAN_MEMBER,
      Self::UnbanMember(_) => discriminator::UNBAN_MEMBER,
      Self::PromoteAdmin(_) => discriminator::PROMOTE_ADMIN,
      Self::DemoteAdmin(_) => discriminator::DEMOTE_ADMIN,
      Self::NicknameChange(_) => discriminator::NICKNAME_CHANGE,
      Self::RoomAnnouncement(_) => discriminator::ROOM_ANNOUNCEMENT,
      Self::ModerationNotification(_) => discriminator::MODERATION_NOTIFICATION,
    }
  }
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_token_auth_roundtrip() {
    let msg = TokenAuth {
      token: "test_token_123".to_string(),
    };
    let encoded = bitcode::encode(&msg);
    let decoded: TokenAuth = bitcode::decode(&encoded).expect("Failed to decode");
    assert_eq!(msg, decoded);
  }

  #[test]
  fn test_auth_success_roundtrip() {
    let msg = AuthSuccess {
      user_id: UserId::new(),
      username: "alice".to_string(),
    };
    let encoded = bitcode::encode(&msg);
    let decoded: AuthSuccess = bitcode::decode(&encoded).expect("Failed to decode");
    assert_eq!(msg, decoded);
  }

  #[test]
  fn test_auth_failure_roundtrip() {
    let msg = AuthFailure {
      reason: "Invalid token".to_string(),
    };
    let encoded = bitcode::encode(&msg);
    let decoded: AuthFailure = bitcode::decode(&encoded).expect("Failed to decode");
    assert_eq!(msg, decoded);
  }

  #[test]
  fn test_user_list_update_roundtrip() {
    let msg = UserListUpdate {
      users: vec![
        UserInfo {
          user_id: UserId::new(),
          username: "alice".to_string(),
          nickname: "Alice".to_string(),
          status: UserStatus::Online,
          avatar_url: None,
          bio: "Hello".to_string(),
          created_at_nanos: 1_000_000_000,
          last_seen_nanos: 2_000_000_000,
        },
        UserInfo {
          user_id: UserId::new(),
          username: "bob".to_string(),
          nickname: "Bob".to_string(),
          status: UserStatus::Away,
          avatar_url: None,
          bio: String::new(),
          created_at_nanos: 1_500_000_000,
          last_seen_nanos: 2_500_000_000,
        },
      ],
    };
    let encoded = bitcode::encode(&msg);
    let decoded: UserListUpdate = bitcode::decode(&encoded).expect("Failed to decode");
    assert_eq!(msg, decoded);
  }

  #[test]
  fn test_user_status_change_roundtrip() {
    let msg = UserStatusChange {
      user_id: UserId::new(),
      status: UserStatus::Busy,
      signature: Some("In a meeting".to_string()),
    };
    let encoded = bitcode::encode(&msg);
    let decoded: UserStatusChange = bitcode::decode(&encoded).expect("Failed to decode");
    assert_eq!(msg, decoded);
  }

  #[test]
  fn test_connection_invite_roundtrip() {
    let msg = ConnectionInvite {
      from: UserId::new(),
      to: UserId::new(),
      note: Some("Let's chat!".to_string()),
    };
    let encoded = bitcode::encode(&msg);
    let decoded: ConnectionInvite = bitcode::decode(&encoded).expect("Failed to decode");
    assert_eq!(msg, decoded);
  }

  #[test]
  fn test_sdp_offer_roundtrip() {
    let msg = SdpOffer {
      from: UserId::new(),
      to: UserId::new(),
      sdp: "v=0\r\no=- 123456 123456 IN IP4 127.0.0.1\r\n".to_string(),
    };
    let encoded = bitcode::encode(&msg);
    let decoded: SdpOffer = bitcode::decode(&encoded).expect("Failed to decode");
    assert_eq!(msg, decoded);
  }

  #[test]
  fn test_ice_candidate_roundtrip() {
    let msg = IceCandidate {
      from: UserId::new(),
      to: UserId::new(),
      candidate: "candidate:1 1 UDP 2122260223 192.168.1.1 54321 typ host".to_string(),
    };
    let encoded = bitcode::encode(&msg);
    let decoded: IceCandidate = bitcode::decode(&encoded).expect("Failed to decode");
    assert_eq!(msg, decoded);
  }

  #[test]
  fn test_peer_established_roundtrip() {
    let msg = PeerEstablished {
      from: UserId::new(),
      to: UserId::new(),
    };
    let encoded = bitcode::encode(&msg);
    let decoded: PeerEstablished = bitcode::decode(&encoded).expect("Failed to decode");
    assert_eq!(msg, decoded);
  }

  #[test]
  fn test_active_peers_list_roundtrip() {
    let msg = ActivePeersList {
      peers: vec![UserId::new(), UserId::new(), UserId::new()],
    };
    let encoded = bitcode::encode(&msg);
    let decoded: ActivePeersList = bitcode::decode(&encoded).expect("Failed to decode");
    assert_eq!(msg, decoded);
  }

  #[test]
  fn test_create_room_roundtrip() {
    let msg = CreateRoom {
      name: "My Room".to_string(),
      room_type: RoomType::Chat,
      password: Some("secret".to_string()),
      max_participants: 8,
    };
    let encoded = bitcode::encode(&msg);
    let decoded: CreateRoom = bitcode::decode(&encoded).expect("Failed to decode");
    assert_eq!(msg, decoded);
  }

  #[test]
  fn test_join_room_roundtrip() {
    let msg = JoinRoom {
      room_id: RoomId::new(),
      password: Some("secret".to_string()),
    };
    let encoded = bitcode::encode(&msg);
    let decoded: JoinRoom = bitcode::decode(&encoded).expect("Failed to decode");
    assert_eq!(msg, decoded);
  }

  #[test]
  fn test_kick_member_roundtrip() {
    let msg = KickMember {
      room_id: RoomId::new(),
      target: UserId::new(),
    };
    let encoded = bitcode::encode(&msg);
    let decoded: KickMember = bitcode::decode(&encoded).expect("Failed to decode");
    assert_eq!(msg, decoded);
  }

  #[test]
  fn test_call_invite_roundtrip() {
    let msg = CallInvite {
      room_id: RoomId::new(),
      media_type: MediaType::Video,
    };
    let encoded = bitcode::encode(&msg);
    let decoded: CallInvite = bitcode::decode(&encoded).expect("Failed to decode");
    assert_eq!(msg, decoded);
  }

  #[test]
  fn test_mute_member_roundtrip() {
    let msg = MuteMember {
      room_id: RoomId::new(),
      target: UserId::new(),
      duration_secs: Some(300),
    };
    let encoded = bitcode::encode(&msg);
    let decoded: MuteMember = bitcode::decode(&encoded).expect("Failed to decode");
    assert_eq!(msg, decoded);
  }

  #[test]
  fn test_ban_member_roundtrip() {
    let msg = BanMember {
      room_id: RoomId::new(),
      target: UserId::new(),
    };
    let encoded = bitcode::encode(&msg);
    let decoded: BanMember = bitcode::decode(&encoded).expect("Failed to decode");
    assert_eq!(msg, decoded);
  }

  #[test]
  fn test_promote_admin_roundtrip() {
    let msg = PromoteAdmin {
      room_id: RoomId::new(),
      target: UserId::new(),
    };
    let encoded = bitcode::encode(&msg);
    let decoded: PromoteAdmin = bitcode::decode(&encoded).expect("Failed to decode");
    assert_eq!(msg, decoded);
  }

  #[test]
  fn test_nickname_change_roundtrip() {
    let msg = NicknameChange {
      user_id: UserId::new(),
      new_nickname: "New Nick".to_string(),
    };
    let encoded = bitcode::encode(&msg);
    let decoded: NicknameChange = bitcode::decode(&encoded).expect("Failed to decode");
    assert_eq!(msg, decoded);
  }

  #[test]
  fn test_moderation_notification_roundtrip() {
    let msg = ModerationNotification {
      room_id: RoomId::new(),
      action: ModerationAction::Kicked,
      target: UserId::new(),
      actor: UserId::new(),
      reason: Some("Spam".to_string()),
    };
    let encoded = bitcode::encode(&msg);
    let decoded: ModerationNotification = bitcode::decode(&encoded).expect("Failed to decode");
    assert_eq!(msg, decoded);
  }

  #[test]
  fn test_signaling_message_discriminator() {
    let msg = SignalingMessage::TokenAuth(TokenAuth {
      token: "test".to_string(),
    });
    assert_eq!(msg.discriminator(), 0x00);

    let msg = SignalingMessage::AuthSuccess(AuthSuccess {
      user_id: UserId::new(),
      username: "alice".to_string(),
    });
    assert_eq!(msg.discriminator(), 0x01);

    let msg = SignalingMessage::SdpOffer(SdpOffer {
      from: UserId::new(),
      to: UserId::new(),
      sdp: "test".to_string(),
    });
    assert_eq!(msg.discriminator(), 0x30);

    let msg = SignalingMessage::MuteMember(MuteMember {
      room_id: RoomId::new(),
      target: UserId::new(),
      duration_secs: None,
    });
    assert_eq!(msg.discriminator(), 0x75);
  }

  #[test]
  fn test_signaling_message_roundtrip() {
    let msg = SignalingMessage::CreateRoom(CreateRoom {
      name: "Test Room".to_string(),
      room_type: RoomType::Theater,
      password: None,
      max_participants: 8,
    });
    let encoded = bitcode::encode(&msg);
    let decoded: SignalingMessage = bitcode::decode(&encoded).expect("Failed to decode");
    assert_eq!(msg, decoded);
  }

  #[test]
  fn test_session_invalidated_roundtrip() {
    let msg = SessionInvalidated {};
    let encoded = bitcode::encode(&msg);
    let decoded: SessionInvalidated = bitcode::decode(&encoded).expect("Failed to decode");
    assert_eq!(msg, decoded);
  }

  #[test]
  fn test_session_invalidated_discriminator() {
    let msg = SignalingMessage::SessionInvalidated(SessionInvalidated {});
    assert_eq!(msg.discriminator(), 0x07);
  }
}
