//! Chat message type definitions
//!
//! ## Chunked Transfer
//!
//! All chat messages are sent wrapped in [`crate::envelope::Envelope`].
//! When the serialized Envelope exceeds [`crate::envelope::DEFAULT_CHUNK_THRESHOLD`],
//! the sender automatically splits it into multiple chunk fragments for transmission.
//! Therefore, large messages like Text, Voice, and Image do not need manual chunking at this layer.

use serde::{Deserialize, Serialize};

use crate::types::{Id, MessageState, Timestamp, gen_id, now_timestamp};

/// Maximum text message length (character count)
///
/// Text exceeding this length should be truncated or rejected at the UI layer.
pub const MAX_TEXT_LENGTH: usize = 50_000;

/// Maximum voice message inline data size (bytes)
///
/// Voice data exceeding this size can still be transmitted via Envelope chunking,
/// but it is recommended to limit recording duration at the UI layer to control data volume.
pub const MAX_VOICE_INLINE_SIZE: usize = 5 * 1024 * 1024; // 5MB

/// Maximum thumbnail size (bytes)
///
/// Image message thumbnails should be compressed to within this size,
/// ensuring the thumbnail message itself does not trigger chunking.
pub const MAX_THUMBNAIL_SIZE: usize = 8 * 1024; // 8KB

/// Chat message content
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MessageContent {
  /// Text message (supports Markdown)
  ///
  /// Recommended length not to exceed [`MAX_TEXT_LENGTH`].
  /// Overly long text will be automatically split and transmitted via Envelope chunking.
  Text(String),

  /// Sticker message
  Sticker {
    /// Sticker pack ID
    pack_id: String,
    /// Sticker ID
    sticker_id: String,
  },

  /// Voice message
  ///
  /// Audio data is directly inline in the message, automatically split and transmitted via Envelope chunking.
  /// Recommended data size not to exceed [`MAX_VOICE_INLINE_SIZE`].
  Voice {
    /// Opus encoded audio data
    data: Vec<u8>,
    /// Duration (milliseconds)
    duration_ms: u32,
  },

  /// Image message
  ///
  /// Thumbnail should be compressed to within [`MAX_THUMBNAIL_SIZE`].
  /// When `full_data` carries the original image data, the entire Envelope will be automatically split via chunking.
  Image {
    /// Thumbnail data (recommended ≤ [`MAX_THUMBNAIL_SIZE`])
    thumbnail: Vec<u8>,
    /// Original image metadata
    meta: ImageMeta,
    /// Original image data (optional, automatically transmitted via Envelope chunking)
    full_data: Option<Vec<u8>>,
  },

  /// File message (reference, actual data transmitted via chunking)
  File(FileMeta),

  /// System notification message
  System(String),
}

/// Image metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageMeta {
  pub width: u32,
  pub height: u32,
  /// Original image size (bytes)
  pub size: u64,
  /// Format (jpeg / png / webp / gif)
  pub format: String,
}

/// File metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileMeta {
  /// File name
  pub name: String,
  /// File size (bytes)
  pub size: u64,
  /// MIME type
  pub mime_type: String,
  /// Associated transfer ID (for chunked transfer tracking)
  pub transfer_id: Option<Id>,
}

/// Single chat message
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
  /// Message unique ID
  pub id: Id,
  /// Sender user ID
  pub from: Id,
  /// Recipient list (multiple for group chat)
  pub to: Vec<Id>,
  /// Message content
  pub content: MessageContent,
  /// Send timestamp
  pub timestamp: Timestamp,
  /// Message state
  pub state: MessageState,
  /// Replied message ID (optional)
  pub reply_to: Option<Id>,
  /// Mentioned user ID list
  pub mentions: Vec<Id>,
}

impl ChatMessage {
  /// Create a new text message
  #[must_use]
  pub fn new_text(from: Id, to: Vec<Id>, text: String) -> Self {
    Self {
      id: gen_id(),
      from,
      to,
      content: MessageContent::Text(text),
      timestamp: now_timestamp(),
      state: MessageState::Sending,
      reply_to: None,
      mentions: Vec::new(),
    }
  }

  /// Create a new sticker message
  #[must_use]
  pub fn new_sticker(from: Id, to: Vec<Id>, pack_id: String, sticker_id: String) -> Self {
    Self {
      id: gen_id(),
      from,
      to,
      content: MessageContent::Sticker {
        pack_id,
        sticker_id,
      },
      timestamp: now_timestamp(),
      state: MessageState::Sending,
      reply_to: None,
      mentions: Vec::new(),
    }
  }

  /// Create a new voice message
  #[must_use]
  pub fn new_voice(from: Id, to: Vec<Id>, data: Vec<u8>, duration_ms: u32) -> Self {
    Self {
      id: gen_id(),
      from,
      to,
      content: MessageContent::Voice { data, duration_ms },
      timestamp: now_timestamp(),
      state: MessageState::Sending,
      reply_to: None,
      mentions: Vec::new(),
    }
  }

  /// Create a new image message
  #[must_use]
  pub fn new_image(from: Id, to: Vec<Id>, thumbnail: Vec<u8>, meta: ImageMeta) -> Self {
    Self {
      id: gen_id(),
      from,
      to,
      content: MessageContent::Image {
        thumbnail,
        meta,
        full_data: None,
      },
      timestamp: now_timestamp(),
      state: MessageState::Sending,
      reply_to: None,
      mentions: Vec::new(),
    }
  }
}

// =============================================================================
// Size Limit Validation
// =============================================================================

/// Message size validation error
#[derive(Debug, Clone, thiserror::Error)]
pub enum MessageSizeError {
  /// Text exceeds maximum length
  #[error("Text length {actual} exceeds maximum limit {max}")]
  TextTooLong {
    /// Actual length
    actual: usize,
    /// Maximum limit
    max: usize,
  },
  /// Voice data exceeds maximum size
  #[error("Voice data size {actual} bytes exceeds maximum limit {max} bytes")]
  VoiceTooLarge {
    /// Actual size
    actual: usize,
    /// Maximum limit
    max: usize,
  },
  /// Thumbnail exceeds maximum size
  #[error("Thumbnail size {actual} bytes exceeds maximum limit {max} bytes")]
  ThumbnailTooLarge {
    /// Actual size
    actual: usize,
    /// Maximum limit
    max: usize,
  },
}

/// Validate text message length
///
/// # Errors
///
/// Returns error when text length exceeds [`MAX_TEXT_LENGTH`].
pub fn validate_text(text: &str) -> Result<(), MessageSizeError> {
  let len = text.chars().count();
  if len > MAX_TEXT_LENGTH {
    return Err(MessageSizeError::TextTooLong {
      actual: len,
      max: MAX_TEXT_LENGTH,
    });
  }
  Ok(())
}

/// Validate voice data size
///
/// # Errors
///
/// Returns error when data size exceeds [`MAX_VOICE_INLINE_SIZE`].
pub fn validate_voice_data(data: &[u8]) -> Result<(), MessageSizeError> {
  if data.len() > MAX_VOICE_INLINE_SIZE {
    return Err(MessageSizeError::VoiceTooLarge {
      actual: data.len(),
      max: MAX_VOICE_INLINE_SIZE,
    });
  }
  Ok(())
}

/// Validate thumbnail size
///
/// # Errors
///
/// Returns error when data size exceeds [`MAX_THUMBNAIL_SIZE`].
pub fn validate_thumbnail(data: &[u8]) -> Result<(), MessageSizeError> {
  if data.len() > MAX_THUMBNAIL_SIZE {
    return Err(MessageSizeError::ThumbnailTooLarge {
      actual: data.len(),
      max: MAX_THUMBNAIL_SIZE,
    });
  }
  Ok(())
}

/// Typing indicator notification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypingIndicator {
  /// User ID who is typing
  pub user_id: Id,
  /// Whether currently typing
  pub is_typing: bool,
}

#[cfg(test)]
mod tests {
  use super::*;

  // ========================================================================
  // Text validation tests
  // ========================================================================

  #[test]
  fn test_validate_text_ok() {
    let text = "A".repeat(MAX_TEXT_LENGTH);
    assert!(validate_text(&text).is_ok());
  }

  #[test]
  fn test_validate_text_too_long() {
    let text = "A".repeat(MAX_TEXT_LENGTH + 1);
    let err = validate_text(&text).unwrap_err();
    assert!(matches!(err, MessageSizeError::TextTooLong { .. }));
  }

  #[test]
  fn test_validate_text_unicode() {
    // Unicode characters each occupy 1 char, confirm counting by characters not bytes
    let text = "Ñ".repeat(MAX_TEXT_LENGTH);
    assert!(validate_text(&text).is_ok());

    let text = "Ñ".repeat(MAX_TEXT_LENGTH + 1);
    assert!(validate_text(&text).is_err());
  }

  #[test]
  fn test_validate_text_empty() {
    assert!(validate_text("").is_ok());
  }

  // ========================================================================
  // Voice validation tests
  // ========================================================================

  #[test]
  fn test_validate_voice_data_ok() {
    let data = vec![0u8; MAX_VOICE_INLINE_SIZE];
    assert!(validate_voice_data(&data).is_ok());
  }

  #[test]
  fn test_validate_voice_data_too_large() {
    let data = vec![0u8; MAX_VOICE_INLINE_SIZE + 1];
    let err = validate_voice_data(&data).unwrap_err();
    assert!(matches!(err, MessageSizeError::VoiceTooLarge { .. }));
  }

  #[test]
  fn test_validate_voice_data_empty() {
    assert!(validate_voice_data(&[]).is_ok());
  }

  // ========================================================================
  // Thumbnail validation tests
  // ========================================================================

  #[test]
  fn test_validate_thumbnail_ok() {
    let data = vec![0xFFu8; MAX_THUMBNAIL_SIZE];
    assert!(validate_thumbnail(&data).is_ok());
  }

  #[test]
  fn test_validate_thumbnail_too_large() {
    let data = vec![0xFFu8; MAX_THUMBNAIL_SIZE + 1];
    let err = validate_thumbnail(&data).unwrap_err();
    assert!(matches!(err, MessageSizeError::ThumbnailTooLarge { .. }));
  }

  // ========================================================================
  // System variant serialization test
  // ========================================================================

  #[test]
  fn test_system_message_serialize_roundtrip() {
    let content = MessageContent::System("User joined the room".to_string());
    let bytes = bitcode::serialize(&content).expect("serialization failed");
    let decoded: MessageContent = bitcode::deserialize(&bytes).expect("deserialization failed");
    if let MessageContent::System(text) = &decoded {
      assert_eq!(text, "User joined the room");
    } else {
      panic!("Message type mismatch");
    }
  }
}
