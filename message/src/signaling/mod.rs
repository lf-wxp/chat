//! Signaling message types for WebSocket communication.
//!
//! This module defines all signaling message types exchanged between client and server
//! over WebSocket. All messages use bitcode binary serialization.

pub mod auth;
pub mod call;
pub mod discriminator;
pub mod invite;
pub mod moderation;
pub mod room;
pub mod user;
pub mod webrtc;

// Re-export all message types at the signaling module level for convenience.
pub use auth::{AuthFailure, AuthSuccess, Ping, Pong, SessionInvalidated, TokenAuth, UserLogout};
pub use call::{CallAccept, CallDecline, CallEnd, CallInvite};
pub use invite::{ConnectionInvite, InviteAccepted, InviteDeclined, InviteTimeout, MultiInvite};
pub use moderation::{
  BanMember, DemoteAdmin, ModerationAction, ModerationNotification, MuteMember, NicknameChange,
  PromoteAdmin, RoomAnnouncement, TheaterMuteAll, TheaterTransferOwner, UnbanMember, UnmuteMember,
};
pub use room::{
  CreateRoom, JoinRoom, KickMember, LeaveRoom, MuteStatusChange, OwnerChanged, RoomCreated,
  RoomJoined, RoomLeft, RoomListUpdate, RoomMemberUpdate, TransferOwnership,
};
pub use user::{UserListUpdate, UserStatusChange};
pub use webrtc::{ActivePeersList, IceCandidate, PeerClosed, PeerEstablished, SdpAnswer, SdpOffer};

use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

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
  /// Room created response.
  RoomCreated(RoomCreated),
  /// Room joined response.
  RoomJoined(RoomJoined),
  /// Room left response.
  RoomLeft(RoomLeft),
  /// Owner changed notification.
  OwnerChanged(OwnerChanged),
  /// Mute status change notification.
  MuteStatusChange(MuteStatusChange),

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
      Self::RoomCreated(_) => discriminator::ROOM_CREATED,
      Self::RoomJoined(_) => discriminator::ROOM_JOINED,
      Self::RoomLeft(_) => discriminator::ROOM_LEFT,
      Self::OwnerChanged(_) => discriminator::OWNER_CHANGED,
      Self::MuteStatusChange(_) => discriminator::MUTE_STATUS_CHANGE,

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
mod tests;
