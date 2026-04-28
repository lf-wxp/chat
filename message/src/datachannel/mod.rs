//! `DataChannel` message types for P2P communication.
//!
//! This module defines all `DataChannel` message types exchanged between peers
//! over WebRTC `DataChannel`. All messages use bitcode binary serialization.

use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::types::{DanmakuPosition, MessageId, RoomId, TransferId, UserId};

// =============================================================================
// Message Type Discriminator Constants
// =============================================================================

/// `DataChannel` message type discriminator values.
///
/// These values are used as the first byte after the magic number (0xBCBC)
/// to identify the message type during deserialization.
/// `DataChannel` messages use discriminators 0x80-0xC3.
pub mod discriminator {
  // Chat Messages (0x80-0x83)
  /// Text chat message type.
  pub const CHAT_TEXT: u8 = 0x80;
  /// Sticker chat message type.
  pub const CHAT_STICKER: u8 = 0x81;
  /// Voice chat message type.
  pub const CHAT_VOICE: u8 = 0x82;
  /// Image chat message type.
  pub const CHAT_IMAGE: u8 = 0x83;

  // File Transfer (0x84-0x86)
  /// File chunk message type.
  pub const FILE_CHUNK: u8 = 0x84;
  /// File metadata message type.
  pub const FILE_METADATA: u8 = 0x85;
  /// File resume request type — receiver asks the sender to replay a
  /// set of missing chunks after reconnect or hash failure (Req 6.6 /
  /// Req 6.5a).
  pub const FILE_RESUME_REQUEST: u8 = 0x86;

  // Message Control (0x90-0x93)
  /// Message acknowledgment type.
  pub const MESSAGE_ACK: u8 = 0x90;
  /// Message revoke type.
  pub const MESSAGE_REVOKE: u8 = 0x91;
  /// Typing indicator type.
  pub const TYPING_INDICATOR: u8 = 0x92;
  /// Message read receipt type.
  pub const MESSAGE_READ: u8 = 0x93;

  // Message Enhancement (0x94-0x95)
  /// Forward message type.
  pub const FORWARD_MESSAGE: u8 = 0x94;
  /// Message reaction type.
  pub const MESSAGE_REACTION: u8 = 0x95;

  // Encryption (0xA0)
  /// ECDH key exchange type.
  pub const ECDH_KEY_EXCHANGE: u8 = 0xA0;

  // Avatar (0xA1-0xA2)
  /// Avatar request type.
  pub const AVATAR_REQUEST: u8 = 0xA1;
  /// Avatar data type.
  pub const AVATAR_DATA: u8 = 0xA2;

  // Theater (0xB0-0xB3)
  /// Danmaku message type.
  pub const DANMAKU: u8 = 0xB0;
  /// Playback progress type.
  pub const PLAYBACK_PROGRESS: u8 = 0xB1;
  /// Subtitle data type.
  pub const SUBTITLE_DATA: u8 = 0xB2;
  /// Subtitle clear type.
  pub const SUBTITLE_CLEAR: u8 = 0xB3;

  // Call-side status broadcasts (0xC0-0xC1)
  /// Local media state broadcast (mic / camera / screen-share flags).
  /// Delivered to every call participant whenever a local toggle fires,
  /// so remote `VideoTile`s can render muted / camera-off icons without
  /// waiting for a heuristic on the incoming RTP track (Req 3.5 / 7.1).
  pub const MEDIA_STATE_UPDATE: u8 = 0xC0;
  /// Peer-side reconnecting status broadcast. Sent when the sender
  /// observes its `RTCPeerConnectionState` leaving `Connected` and
  /// again when it recovers, so the remote UI can show a "reconnecting"
  /// hint without relying on server-observed WebSocket state
  /// (Req 10.5.24).
  pub const RECONNECTING_STATE: u8 = 0xC1;
}

// =============================================================================
// Chat Messages
// =============================================================================

/// Text chat message.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct ChatText {
  /// Unique message ID.
  pub message_id: MessageId,
  /// Message content (Markdown supported).
  pub content: String,
  /// Reply-to message ID (optional).
  pub reply_to: Option<MessageId>,
  /// Sender timestamp in nanoseconds since Unix epoch.
  pub timestamp_nanos: u64,
}

/// Sticker chat message.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct ChatSticker {
  /// Unique message ID.
  pub message_id: MessageId,
  /// Sticker pack ID.
  pub pack_id: String,
  /// Sticker ID within the pack.
  pub sticker_id: String,
  /// Reply-to message ID (optional).
  pub reply_to: Option<MessageId>,
  /// Sender timestamp in nanoseconds since Unix epoch.
  pub timestamp_nanos: u64,
}

/// Voice chat message.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct ChatVoice {
  /// Unique message ID.
  pub message_id: MessageId,
  /// Opus-encoded audio data.
  pub audio_data: Vec<u8>,
  /// Duration in milliseconds.
  pub duration_ms: u32,
  /// Waveform preview (array of amplitude samples).
  pub waveform: Vec<u8>,
  /// Reply-to message ID (optional).
  pub reply_to: Option<MessageId>,
  /// Sender timestamp in nanoseconds since Unix epoch.
  pub timestamp_nanos: u64,
}

/// Image chat message.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct ChatImage {
  /// Unique message ID.
  pub message_id: MessageId,
  /// Image data (JPEG/PNG/WebP).
  pub image_data: Vec<u8>,
  /// Thumbnail data (smaller preview).
  pub thumbnail: Vec<u8>,
  /// Original width in pixels.
  pub width: u32,
  /// Original height in pixels.
  pub height: u32,
  /// Reply-to message ID (optional).
  pub reply_to: Option<MessageId>,
  /// Sender timestamp in nanoseconds since Unix epoch.
  pub timestamp_nanos: u64,
}

// =============================================================================
// File Transfer
// =============================================================================

/// File chunk for chunked file transfer.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct FileChunk {
  /// Transfer ID (shared across all chunks of same file).
  pub transfer_id: TransferId,
  /// Chunk index (0-based).
  pub chunk_index: u32,
  /// Total number of chunks.
  pub total_chunks: u32,
  /// Chunk data (max 64KB).
  pub data: Vec<u8>,
  /// SHA-256 hash of this chunk.
  pub chunk_hash: [u8; 32],
}

/// File metadata sent before file transfer.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct FileMetadata {
  /// Unique message ID.
  pub message_id: MessageId,
  /// Transfer ID (shared across all chunks).
  pub transfer_id: TransferId,
  /// File name.
  pub filename: String,
  /// File size in bytes.
  pub size: u64,
  /// MIME type.
  pub mime_type: String,
  /// SHA-256 hash of entire file.
  pub file_hash: [u8; 32],
  /// Total number of chunks.
  pub total_chunks: u32,
  /// Chunk size in bytes.
  pub chunk_size: u32,
  /// Reply-to message ID (optional).
  pub reply_to: Option<MessageId>,
  /// Sender timestamp in nanoseconds since Unix epoch.
  pub timestamp_nanos: u64,
  /// Room ID when the file is shared inside a room conversation. `None`
  /// for 1:1 chats. Lets the receiver route the inbound file card to
  /// the correct conversation even when the wire frame is delivered
  /// peer-to-peer (Req 2.3 / Req 6.10).
  pub room_id: Option<RoomId>,
}

/// Receiver-initiated resume request.
///
/// Emitted either after a `PeerConnection` re-established (Req 6.6) or
/// after the receiver detected a SHA-256 mismatch (Req 6.5a) and wants
/// the sender to replay the missing chunks. `missing_chunks` is
/// deliberately a plain `Vec<u32>` rather than a bitmap so the sender
/// can iterate in order without decoding helper state; for typical
/// resume scenarios the vector holds a handful of indices at most.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct FileResumeRequest {
  /// Transfer ID this request targets.
  pub transfer_id: TransferId,
  /// Chunk indices the sender should replay. An empty list means "no
  /// chunks missing" (used as a keep-alive / ack); a full `0..total`
  /// list asks for a full retransmit after a hash mismatch.
  pub missing_chunks: Vec<u32>,
  /// Receiver timestamp in nanoseconds since Unix epoch.
  pub timestamp_nanos: u64,
}

// =============================================================================
// Message Control
// =============================================================================

/// Message acknowledgment status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AckStatus {
  /// Message received.
  Received,
  /// Message failed to deliver.
  Failed,
}

/// Message acknowledgment.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct MessageAck {
  /// Acknowledged message ID.
  pub message_id: MessageId,
  /// Acknowledgment status.
  pub status: AckStatus,
  /// Timestamp of acknowledgment in nanoseconds.
  pub timestamp_nanos: u64,
}

/// Message revoke request (within 2 minutes).
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct MessageRevoke {
  /// Message ID to revoke.
  pub message_id: MessageId,
  /// Revocation timestamp in nanoseconds.
  pub timestamp_nanos: u64,
}

/// Typing indicator.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct TypingIndicator {
  /// Whether user is typing.
  pub is_typing: bool,
}

/// Message read receipt.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct MessageRead {
  /// Message IDs that were read.
  pub message_ids: Vec<MessageId>,
  /// Read timestamp in nanoseconds.
  pub timestamp_nanos: u64,
}

// =============================================================================
// Message Enhancement
// =============================================================================

/// Forward message.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct ForwardMessage {
  /// Unique message ID for the forwarded message.
  pub message_id: MessageId,
  /// Original message ID being forwarded.
  pub original_message_id: MessageId,
  /// Original sender user ID.
  pub original_sender: UserId,
  /// Original message content.
  pub content: String,
  /// Forward timestamp in nanoseconds.
  pub timestamp_nanos: u64,
}

/// Reaction action type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReactionAction {
  /// Add reaction.
  Add,
  /// Remove reaction.
  Remove,
}

/// Message reaction.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct MessageReaction {
  /// Message ID to react to.
  pub message_id: MessageId,
  /// Reaction emoji (Unicode or custom sticker ID).
  pub emoji: String,
  /// Action (add or remove).
  pub action: ReactionAction,
  /// Reaction timestamp in nanoseconds.
  pub timestamp_nanos: u64,
}

// =============================================================================
// Encryption
// =============================================================================

/// ECDH public key for key exchange.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct EcdhKeyExchange {
  /// Sender's ECDH public key (P-256 raw uncompressed point, 65 bytes).
  ///
  /// Uses variable-length `Vec<u8>` to accommodate the raw EC point format
  /// (1 prefix byte + 32 X bytes + 32 Y bytes = 65 bytes for P-256).
  pub public_key: Vec<u8>,
  /// Key exchange timestamp in nanoseconds.
  pub timestamp_nanos: u64,
}

// =============================================================================
// Avatar
// =============================================================================

/// Request peer's avatar.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct AvatarRequest {
  /// Requested user ID.
  pub user_id: UserId,
}

/// Avatar data response.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct AvatarData {
  /// User ID.
  pub user_id: UserId,
  /// Avatar image data (JPEG/PNG/WebP).
  pub data: Vec<u8>,
  /// MIME type.
  pub mime_type: String,
  /// Width in pixels.
  pub width: u32,
  /// Height in pixels.
  pub height: u32,
}

// =============================================================================
// Theater
// =============================================================================

/// Danmaku (bullet comment) message.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct Danmaku {
  /// Danmaku content.
  pub content: String,
  /// Font size (12-36).
  pub font_size: u8,
  /// Text color (RGB hex, e.g., 0xFFFFFF).
  pub color: u32,
  /// Display position.
  pub position: DanmakuPosition,
  /// Display timestamp in video (milliseconds).
  pub video_time_ms: u64,
  /// Sender timestamp in nanoseconds.
  pub timestamp_nanos: u64,
}

/// Playback progress synchronization.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct PlaybackProgress {
  /// Room ID.
  pub room_id: RoomId,
  /// Current playback time in milliseconds.
  pub current_time_ms: u64,
  /// Total duration in milliseconds.
  pub duration_ms: u64,
  /// Whether video is paused.
  pub is_paused: bool,
  /// Sync timestamp in nanoseconds.
  pub timestamp_nanos: u64,
}

/// Subtitle data for theater mode.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct SubtitleData {
  /// Room ID.
  pub room_id: RoomId,
  /// Subtitle entries.
  pub entries: Vec<SubtitleEntry>,
}

/// Single subtitle entry.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct SubtitleEntry {
  /// Start time in milliseconds.
  pub start_ms: u32,
  /// End time in milliseconds.
  pub end_ms: u32,
  /// Subtitle text.
  pub text: String,
}

/// Clear subtitle display.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct SubtitleClear {
  /// Room ID.
  pub room_id: RoomId,
}

// =============================================================================
// Call Status Broadcasts (Req 3.5 / 7.1 / 10.5.24)
// =============================================================================

/// Local media state broadcast. Sent whenever a toggle fires on the
/// sender so remote call participants can render muted / camera-off /
/// screen-sharing icons without waiting for heuristics on the incoming
/// RTP tracks (Req 3.5 / 7.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct MediaStateUpdate {
  /// Whether the microphone track is enabled on the sender.
  pub mic_enabled: bool,
  /// Whether the camera track is enabled on the sender.
  pub camera_enabled: bool,
  /// Whether the sender is currently sharing their screen.
  pub screen_sharing: bool,
}

/// Peer-side reconnecting status. Sent by a call participant when its
/// `RTCPeerConnectionState` leaves `Connected` (with `reconnecting =
/// true`) and again once it recovers (`reconnecting = false`). Lets
/// the remote UI render an "other participant is reconnecting" hint
/// during transient network flaps (Req 10.5.24).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct ReconnectingState {
  /// Whether the sender is currently attempting to reconnect.
  pub reconnecting: bool,
}

// =============================================================================
// Unified DataChannel Message Enum
// =============================================================================

/// Unified `DataChannel` message enum.
///
/// This enum wraps all `DataChannel` message types for unified handling.
/// Each variant corresponds to a specific message type discriminator (0x80-0xC3).
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DataChannelMessage {
  // Chat Messages
  /// Text chat message.
  ChatText(ChatText),
  /// Sticker chat message.
  ChatSticker(ChatSticker),
  /// Voice chat message.
  ChatVoice(ChatVoice),
  /// Image chat message.
  ChatImage(ChatImage),

  // File Transfer
  /// File chunk.
  FileChunk(FileChunk),
  /// File metadata.
  FileMetadata(FileMetadata),
  /// File resume request (receiver → sender).
  FileResumeRequest(FileResumeRequest),

  // Message Control
  /// Message acknowledgment.
  MessageAck(MessageAck),
  /// Message revoke.
  MessageRevoke(MessageRevoke),
  /// Typing indicator.
  TypingIndicator(TypingIndicator),
  /// Message read receipt.
  MessageRead(MessageRead),

  // Message Enhancement
  /// Forward message.
  ForwardMessage(ForwardMessage),
  /// Message reaction.
  MessageReaction(MessageReaction),

  // Encryption
  /// ECDH key exchange.
  EcdhKeyExchange(EcdhKeyExchange),

  // Avatar
  /// Avatar request.
  AvatarRequest(AvatarRequest),
  /// Avatar data.
  AvatarData(AvatarData),

  // Theater
  /// Danmaku message.
  Danmaku(Danmaku),
  /// Playback progress.
  PlaybackProgress(PlaybackProgress),
  /// Subtitle data.
  SubtitleData(SubtitleData),
  /// Subtitle clear.
  SubtitleClear(SubtitleClear),

  // Call Status Broadcasts
  /// Local media state (mic / camera / screen-share) broadcast.
  MediaStateUpdate(MediaStateUpdate),
  /// Peer-side reconnecting status broadcast.
  ReconnectingState(ReconnectingState),
}

impl DataChannelMessage {
  /// Returns the message type discriminator for this message.
  #[must_use]
  pub const fn discriminator(&self) -> u8 {
    match self {
      Self::ChatText(_) => discriminator::CHAT_TEXT,
      Self::ChatSticker(_) => discriminator::CHAT_STICKER,
      Self::ChatVoice(_) => discriminator::CHAT_VOICE,
      Self::ChatImage(_) => discriminator::CHAT_IMAGE,

      Self::FileChunk(_) => discriminator::FILE_CHUNK,
      Self::FileMetadata(_) => discriminator::FILE_METADATA,
      Self::FileResumeRequest(_) => discriminator::FILE_RESUME_REQUEST,

      Self::MessageAck(_) => discriminator::MESSAGE_ACK,
      Self::MessageRevoke(_) => discriminator::MESSAGE_REVOKE,
      Self::TypingIndicator(_) => discriminator::TYPING_INDICATOR,
      Self::MessageRead(_) => discriminator::MESSAGE_READ,

      Self::ForwardMessage(_) => discriminator::FORWARD_MESSAGE,
      Self::MessageReaction(_) => discriminator::MESSAGE_REACTION,

      Self::EcdhKeyExchange(_) => discriminator::ECDH_KEY_EXCHANGE,

      Self::AvatarRequest(_) => discriminator::AVATAR_REQUEST,
      Self::AvatarData(_) => discriminator::AVATAR_DATA,

      Self::Danmaku(_) => discriminator::DANMAKU,
      Self::PlaybackProgress(_) => discriminator::PLAYBACK_PROGRESS,
      Self::SubtitleData(_) => discriminator::SUBTITLE_DATA,
      Self::SubtitleClear(_) => discriminator::SUBTITLE_CLEAR,

      Self::MediaStateUpdate(_) => discriminator::MEDIA_STATE_UPDATE,
      Self::ReconnectingState(_) => discriminator::RECONNECTING_STATE,
    }
  }

  /// Returns `true` when the payload is small enough to persist as JSON
  /// in the ACK queue without significant storage bloat. Large binary
  /// payloads (images, voice, file chunks, avatar data) return `false`
  /// so callers can skip JSON serialisation and mark them as
  /// "manual resend required" after page refresh (V4 optimisation).
  #[must_use]
  pub const fn is_lightweight(&self) -> bool {
    matches!(
      self,
      Self::ChatText(_)
        | Self::ChatSticker(_)
        | Self::ForwardMessage(_)
        | Self::MessageAck(_)
        | Self::MessageRevoke(_)
        | Self::TypingIndicator(_)
        | Self::MessageRead(_)
        | Self::MessageReaction(_)
        | Self::EcdhKeyExchange(_)
        | Self::AvatarRequest(_)
        | Self::Danmaku(_)
        | Self::PlaybackProgress(_)
        | Self::SubtitleClear(_)
        | Self::MediaStateUpdate(_)
        | Self::ReconnectingState(_)
        | Self::FileResumeRequest(_)
    )
  }
}

// =============================================================================
// Helper Methods
// =============================================================================

impl SubtitleEntry {
  /// Create a new subtitle entry.
  #[must_use]
  pub const fn new(start_ms: u32, end_ms: u32, text: String) -> Self {
    Self {
      start_ms,
      end_ms,
      text,
    }
  }

  /// Returns the duration of this subtitle entry in milliseconds.
  #[must_use]
  pub const fn duration_ms(&self) -> u32 {
    self.end_ms.saturating_sub(self.start_ms)
  }

  /// Returns true if this subtitle is active at the given timestamp.
  #[must_use]
  pub const fn is_active_at(&self, timestamp_ms: u32) -> bool {
    timestamp_ms >= self.start_ms && timestamp_ms < self.end_ms
  }
}

impl Danmaku {
  /// Maximum allowed content length (100 characters).
  pub const MAX_CONTENT_LENGTH: usize = 100;

  /// Minimum font size.
  pub const MIN_FONT_SIZE: u8 = 12;

  /// Maximum font size.
  pub const MAX_FONT_SIZE: u8 = 36;

  /// Returns true if this danmaku is valid.
  #[must_use]
  pub fn is_valid(&self) -> bool {
    let char_count = self.content.chars().count();
    !self.content.is_empty()
      && char_count <= Self::MAX_CONTENT_LENGTH
      && self.font_size >= Self::MIN_FONT_SIZE
      && self.font_size <= Self::MAX_FONT_SIZE
  }
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests;
