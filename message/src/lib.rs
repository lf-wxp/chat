//! # Message Crate
//!
//! Shared message types and binary protocol for WebRTC Chat Application.
//!
//! This crate provides:
//! - Core data types (`UserId`, `RoomId`, `MessageId`, etc.)
//! - Signaling messages (WebSocket communication)
//! - `DataChannel` messages (P2P communication)
//! - Binary protocol frame structure
//! - Error code system with i18n keys
//!
//! ## Feature Flags
//!
//! - `native` - Enable native target support (default)
//! - `wasm` - Enable WebAssembly target support

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]
#![warn(unreachable_pub)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]

pub mod datachannel;
pub mod error;
pub mod frame;
pub mod signaling;
pub mod types;

// WASM bindings module (only for wasm32 target)
#[cfg(target_arch = "wasm32")]
pub mod wasm;

// Re-export commonly used types at crate root
pub use datachannel::{
  AckStatus, AvatarData, AvatarRequest, ChatImage, ChatSticker, ChatText, ChatVoice, Danmaku,
  DataChannelMessage, EcdhKeyExchange, FileChunk, FileMetadata, ForwardMessage, MessageAck,
  MessageReaction, MessageRead, MessageRevoke, PlaybackProgress, ReactionAction, SubtitleClear,
  SubtitleData, SubtitleEntry, TypingIndicator,
};
pub use error::{ErrorCode, ErrorResponse};
pub use frame::{
  ChunkBitmap, ChunkHeader, ChunkManager, ChunkedMessage, MAGIC_NUMBER, MAX_CHUNK_SIZE,
  MessageFrame, ReassemblyBuffer, decode_frame, encode_frame,
};
pub use signaling::{
  ActivePeersList, AuthFailure, AuthSuccess, BanMember, CallAccept, CallDecline, CallEnd,
  CallInvite, ConnectionInvite, CreateRoom, DemoteAdmin, IceCandidate, InviteAccepted,
  InviteDeclined, InviteTimeout, JoinRoom, KickMember, ModerationAction, ModerationNotification,
  MultiInvite, MuteMember, NicknameChange, PeerClosed, PeerEstablished, Ping, Pong, PromoteAdmin,
  RoomAnnouncement, RoomListUpdate, RoomMemberUpdate, SdpAnswer, SdpOffer, SignalingMessage,
  TheaterMuteAll, TheaterTransferOwner, TokenAuth, TransferOwnership, UnbanMember, UnmuteMember,
  UserListUpdate, UserLogout, UserStatusChange,
};
pub use types::{MessageId, RoomId, TransferId, UserId};
