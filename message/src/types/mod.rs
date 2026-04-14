//! Core data types for WebRTC Chat Application.
//!
//! This module defines all fundamental types used throughout the application,
//! including identifiers, enums, and structs for users, rooms, and messages.

pub mod enums;
pub mod identifiers;
pub mod mute;
pub mod role;
pub mod structs;

// Re-export all public types for backward compatibility
pub use enums::{
  DanmakuPosition, MediaType, MessageContentType, NetworkQuality, ReactionAction, RoomType,
  UserStatus,
};
pub use identifiers::{MessageId, RoomId, TransferId, UserId};
pub use mute::MuteInfo;
pub use role::RoomRole;
pub use structs::{ImageMeta, MemberInfo, RoomInfo, UserInfo};

// SubtitleEntry is defined in datachannel module and re-exported here
// for backward compatibility. The datachannel version uses u32 timestamps
// which is sufficient for ~49 days of millisecond precision.
pub use crate::datachannel::SubtitleEntry;

#[cfg(test)]
mod tests;
