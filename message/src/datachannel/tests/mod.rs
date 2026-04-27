//! `DataChannel` message tests module.
//!
//! Tests are organized by functionality:
//! - `roundtrip`: Bitcode roundtrip tests for each message type
//! - `discriminator`: Discriminator value and uniqueness tests
//! - `validation`: Danmaku, `SubtitleEntry`, and boundary validation tests
//! - `serialization`: JSON serialization roundtrip tests

mod discriminator_test;
mod roundtrip;
mod serialization;
mod validation;

// Re-export all necessary types for test submodules
pub(super) use super::{
  AckStatus, AvatarData, AvatarRequest, ChatImage, ChatSticker, ChatText, ChatVoice, Danmaku,
  DanmakuPosition, DataChannelMessage, EcdhKeyExchange, FileChunk, FileMetadata, ForwardMessage,
  MediaStateUpdate, MessageAck, MessageReaction, MessageRead, MessageRevoke, PlaybackProgress,
  ReactionAction, ReconnectingState, SubtitleClear, SubtitleData, SubtitleEntry, TypingIndicator,
  discriminator,
};

pub(super) use crate::types::{MessageId, RoomId, TransferId, UserId};

/// Helper: test bitcode encode→decode roundtrip for a value.
pub(super) fn test_bitcode_roundtrip<
  T: bitcode::Encode + for<'a> bitcode::Decode<'a> + PartialEq + std::fmt::Debug,
>(
  value: &T,
) {
  let encoded = bitcode::encode(value);
  let decoded: T = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(value, &decoded, "Roundtrip failed for {value:?}");
}
