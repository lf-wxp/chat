//! Signaling message type discriminator constants.
//!
//! These values are used as the first byte after the magic number (0xBCBC)
//! to identify the message type during deserialization.

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

// Room Response Messages (0x57-0x5A)
/// Room created response message type.
pub const ROOM_CREATED: u8 = 0x57;
/// Room joined response message type.
pub const ROOM_JOINED: u8 = 0x58;
/// Room left response message type.
pub const ROOM_LEFT: u8 = 0x59;
/// Owner changed message type.
pub const OWNER_CHANGED: u8 = 0x5A;
/// Mute status change message type.
pub const MUTE_STATUS_CHANGE: u8 = 0x5B;
