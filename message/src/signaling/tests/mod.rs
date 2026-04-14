//! Signaling message tests module.
//!
//! Tests are organized by message category:
//! - `auth`: Authentication messages (`TokenAuth`, `AuthSuccess`, etc.)
//! - `user`: User discovery messages (`UserListUpdate`, `UserStatusChange`)
//! - `invite`: Connection invite messages
//! - `webrtc`: WebRTC signaling messages (SDP, ICE, Peer)
//! - `room`: Room management messages
//! - `call`: Call control messages
//! - `moderation`: Moderation action messages
//! - `discriminator`: Discriminator value and uniqueness tests

mod auth;
mod call;
mod discriminator;
mod invite;
mod moderation;
mod room;
mod user;
mod webrtc;

// Re-export all necessary types for test submodules
pub(super) use super::{
  // WebRTC messages
  ActivePeersList,
  // Auth messages
  AuthFailure,
  AuthSuccess,
  // Moderation messages
  BanMember,
  // Call messages
  CallAccept,
  CallDecline,
  CallEnd,
  CallInvite,
  // Invite messages
  ConnectionInvite,
  // Room messages
  CreateRoom,
  DemoteAdmin,
  IceCandidate,
  InviteAccepted,
  InviteDeclined,
  InviteTimeout,
  JoinRoom,
  KickMember,
  LeaveRoom,
  ModerationNotification,
  MultiInvite,
  MuteMember,
  MuteStatusChange,
  NicknameChange,
  OwnerChanged,
  PeerClosed,
  PeerEstablished,
  Ping,
  Pong,
  PromoteAdmin,
  RoomAnnouncement,
  RoomCreated,
  RoomJoined,
  RoomLeft,
  RoomListUpdate,
  RoomMemberUpdate,
  SdpAnswer,
  SdpOffer,
  SessionInvalidated,
  // Main enum
  SignalingMessage,
  TheaterMuteAll,
  TheaterTransferOwner,
  TokenAuth,
  TransferOwnership,
  UnbanMember,
  UnmuteMember,
  // User messages
  UserListUpdate,
  UserLogout,
  UserStatusChange,
};

// Re-export frame types for encoding/decoding tests
pub(super) use crate::frame::{MessageFrame, decode_frame, encode_frame};

// Re-export common types
pub(super) use crate::types::{MediaType, MemberInfo, RoomId, RoomInfo, RoomRole, RoomType};

// Re-export error codes for tests
pub(super) use crate::error::codes::SIG001;

// Re-export discriminator constants
pub(super) use crate::signaling::discriminator::{
  ACTIVE_PEERS_LIST, AUTH_FAILURE, AUTH_SUCCESS, BAN_MEMBER, CALL_ACCEPT, CALL_DECLINE, CALL_END,
  CALL_INVITE, CONNECTION_INVITE, CREATE_ROOM, DEMOTE_ADMIN, ERROR_RESPONSE, ICE_CANDIDATE,
  INVITE_ACCEPTED, INVITE_DECLINED, INVITE_TIMEOUT, JOIN_ROOM, KICK_MEMBER, LEAVE_ROOM,
  MODERATION_NOTIFICATION, MULTI_INVITE, MUTE_MEMBER, MUTE_STATUS_CHANGE, NICKNAME_CHANGE,
  OWNER_CHANGED, PEER_CLOSED, PEER_ESTABLISHED, PING, PONG, PROMOTE_ADMIN, ROOM_CREATED,
  ROOM_JOINED, ROOM_LEFT, ROOM_LIST_UPDATE, ROOM_MEMBER_UPDATE, SDP_ANSWER, SDP_OFFER,
  SESSION_INVALIDATED, THEATER_MUTE_ALL, THEATER_TRANSFER_OWNER, TOKEN_AUTH, TRANSFER_OWNERSHIP,
  UNBAN_MEMBER, UNMUTE_MEMBER, USER_LIST_UPDATE, USER_LOGOUT, USER_STATUS_CHANGE,
};

// Re-export specific types used by individual test files
pub(super) use crate::types::{UserId, UserStatus};

// Re-export ModerationAction for moderation tests
pub(super) use crate::ModerationAction;

// Re-export ErrorResponse for error tests
pub(super) use crate::error::ErrorResponse;
