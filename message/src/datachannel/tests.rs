use super::*;
use crate::types::MessageId;

fn test_bitcode_roundtrip<T: Encode + for<'a> Decode<'a> + PartialEq + std::fmt::Debug>(value: &T) {
  let encoded = bitcode::encode(value);
  let decoded: T = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(value, &decoded, "Roundtrip failed for {value:?}");
}

#[test]
fn test_chat_text_roundtrip() {
  let msg = ChatText {
    message_id: MessageId::new(),
    content: "Hello, **world**!".to_string(),
    reply_to: Some(MessageId::new()),
    timestamp_nanos: 1_000_000_000,
  };
  test_bitcode_roundtrip(&msg);
}

#[test]
fn test_chat_sticker_roundtrip() {
  let msg = ChatSticker {
    message_id: MessageId::new(),
    pack_id: "pack_001".to_string(),
    sticker_id: "sticker_123".to_string(),
    reply_to: None,
    timestamp_nanos: 1_000_000_000,
  };
  test_bitcode_roundtrip(&msg);
}

#[test]
fn test_chat_voice_roundtrip() {
  let msg = ChatVoice {
    message_id: MessageId::new(),
    audio_data: vec![0u8; 100],
    duration_ms: 5000,
    waveform: vec![10, 20, 30, 40, 50],
    reply_to: None,
    timestamp_nanos: 1_000_000_000,
  };
  test_bitcode_roundtrip(&msg);
}

#[test]
fn test_chat_image_roundtrip() {
  let msg = ChatImage {
    message_id: MessageId::new(),
    image_data: vec![0xFF; 1000],
    thumbnail: vec![0xAA; 100],
    width: 1920,
    height: 1080,
    reply_to: Some(MessageId::new()),
    timestamp_nanos: 1_000_000_000,
  };
  test_bitcode_roundtrip(&msg);
}

#[test]
fn test_file_chunk_roundtrip() {
  let msg = FileChunk {
    transfer_id: TransferId::new(),
    chunk_index: 0,
    total_chunks: 10,
    data: vec![0xAB; 1024],
    chunk_hash: [0u8; 32],
  };
  test_bitcode_roundtrip(&msg);
}

#[test]
fn test_file_metadata_roundtrip() {
  let msg = FileMetadata {
    message_id: MessageId::new(),
    transfer_id: TransferId::new(),
    filename: "document.pdf".to_string(),
    size: 1_024_000,
    mime_type: "application/pdf".to_string(),
    file_hash: [0u8; 32],
    total_chunks: 16,
    chunk_size: 65536,
    reply_to: None,
    timestamp_nanos: 1_000_000_000,
  };
  test_bitcode_roundtrip(&msg);
}

#[test]
fn test_message_ack_roundtrip() {
  let msg = MessageAck {
    message_id: MessageId::new(),
    status: AckStatus::Received,
    timestamp_nanos: 1_000_000_000,
  };
  test_bitcode_roundtrip(&msg);
}

#[test]
fn test_message_revoke_roundtrip() {
  let msg = MessageRevoke {
    message_id: MessageId::new(),
    timestamp_nanos: 1_000_000_000,
  };
  test_bitcode_roundtrip(&msg);
}

#[test]
fn test_typing_indicator_roundtrip() {
  let msg = TypingIndicator { is_typing: true };
  test_bitcode_roundtrip(&msg);
}

#[test]
fn test_message_read_roundtrip() {
  let msg = MessageRead {
    message_ids: vec![MessageId::new(), MessageId::new()],
    timestamp_nanos: 1_000_000_000,
  };
  test_bitcode_roundtrip(&msg);
}

#[test]
fn test_forward_message_roundtrip() {
  let msg = ForwardMessage {
    message_id: MessageId::new(),
    original_message_id: MessageId::new(),
    original_sender: UserId::new(),
    content: "Forwarded content".to_string(),
    timestamp_nanos: 1_000_000_000,
  };
  test_bitcode_roundtrip(&msg);
}

#[test]
fn test_message_reaction_roundtrip() {
  let msg = MessageReaction {
    message_id: MessageId::new(),
    emoji: "👍".to_string(),
    action: ReactionAction::Add,
    timestamp_nanos: 1_000_000_000,
  };
  test_bitcode_roundtrip(&msg);
}

#[test]
fn test_ecdh_key_exchange_roundtrip() {
  let msg = EcdhKeyExchange {
    public_key: vec![0xAB; 65], // P-256 raw format: 65 bytes
    timestamp_nanos: 1_000_000_000,
  };
  test_bitcode_roundtrip(&msg);
}

#[test]
fn test_avatar_request_roundtrip() {
  let msg = AvatarRequest {
    user_id: UserId::new(),
  };
  test_bitcode_roundtrip(&msg);
}

#[test]
fn test_avatar_data_roundtrip() {
  let msg = AvatarData {
    user_id: UserId::new(),
    data: vec![0xFF; 500],
    mime_type: "image/jpeg".to_string(),
    width: 128,
    height: 128,
  };
  test_bitcode_roundtrip(&msg);
}

#[test]
fn test_danmaku_roundtrip() {
  let msg = Danmaku {
    content: "This is awesome!".to_string(),
    font_size: 24,
    color: 0xFF_FF_FF,
    position: DanmakuPosition::Scroll,
    video_time_ms: 5000,
    timestamp_nanos: 1_000_000_000,
  };
  test_bitcode_roundtrip(&msg);
}

#[test]
fn test_playback_progress_roundtrip() {
  let msg = PlaybackProgress {
    room_id: RoomId::new(),
    current_time_ms: 30000,
    duration_ms: 3_600_000,
    is_paused: false,
    timestamp_nanos: 1_000_000_000,
  };
  test_bitcode_roundtrip(&msg);
}

#[test]
fn test_subtitle_data_roundtrip() {
  let msg = SubtitleData {
    room_id: RoomId::new(),
    entries: vec![
      SubtitleEntry {
        start_ms: 0,
        end_ms: 3000,
        text: "Hello, world!".to_string(),
      },
      SubtitleEntry {
        start_ms: 3000,
        end_ms: 6000,
        text: "Goodbye!".to_string(),
      },
    ],
  };
  test_bitcode_roundtrip(&msg);
}

#[test]
fn test_subtitle_clear_roundtrip() {
  let msg = SubtitleClear {
    room_id: RoomId::new(),
  };
  test_bitcode_roundtrip(&msg);
}

#[test]
fn test_datachannel_message_discriminator() {
  // Chat messages
  assert_eq!(
    DataChannelMessage::ChatText(ChatText {
      message_id: MessageId::new(),
      content: String::new(),
      reply_to: None,
      timestamp_nanos: 0,
    })
    .discriminator(),
    0x80
  );

  assert_eq!(
    DataChannelMessage::ChatSticker(ChatSticker {
      message_id: MessageId::new(),
      pack_id: String::new(),
      sticker_id: String::new(),
      reply_to: None,
      timestamp_nanos: 0,
    })
    .discriminator(),
    0x81
  );

  // File transfer
  assert_eq!(
    DataChannelMessage::FileMetadata(FileMetadata {
      message_id: MessageId::new(),
      transfer_id: TransferId::new(),
      filename: String::new(),
      size: 0,
      mime_type: String::new(),
      file_hash: [0; 32],
      total_chunks: 0,
      chunk_size: 0,
      reply_to: None,
      timestamp_nanos: 0,
    })
    .discriminator(),
    0x85
  );

  // Message control
  assert_eq!(
    DataChannelMessage::MessageAck(MessageAck {
      message_id: MessageId::new(),
      status: AckStatus::Received,
      timestamp_nanos: 0,
    })
    .discriminator(),
    0x90
  );

  // Theater
  assert_eq!(
    DataChannelMessage::Danmaku(Danmaku {
      content: String::new(),
      font_size: 24,
      color: 0xFF_FF_FF,
      position: DanmakuPosition::Scroll,
      video_time_ms: 0,
      timestamp_nanos: 0,
    })
    .discriminator(),
    0xB0
  );
}

#[test]
fn test_datachannel_message_roundtrip() {
  let msg = DataChannelMessage::ChatText(ChatText {
    message_id: MessageId::new(),
    content: "Test message".to_string(),
    reply_to: None,
    timestamp_nanos: 1_000_000_000,
  });
  let encoded = bitcode::encode(&msg);
  let decoded: DataChannelMessage = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_subtitle_entry_creation() {
  let entry = SubtitleEntry::new(1000, 3000, "Hello, world!".to_string());
  assert_eq!(entry.start_ms, 1000);
  assert_eq!(entry.end_ms, 3000);
  assert_eq!(entry.text, "Hello, world!");
  assert_eq!(entry.duration_ms(), 2000);
  assert!(entry.is_active_at(1500));
  assert!(!entry.is_active_at(500));
  assert!(!entry.is_active_at(4000));
}

#[test]
fn test_danmaku_validation() {
  // Valid danmaku
  let danmaku = Danmaku {
    content: "Test".to_string(),
    font_size: 24,
    color: 0x00FF_FFFF,
    position: DanmakuPosition::Scroll,
    video_time_ms: 0,
    timestamp_nanos: 0,
  };
  assert!(danmaku.is_valid());

  // Invalid font size (too small)
  let invalid = Danmaku {
    content: "Test".to_string(),
    font_size: 10,
    color: 0x00FF_FFFF,
    position: DanmakuPosition::Scroll,
    video_time_ms: 0,
    timestamp_nanos: 0,
  };
  assert!(!invalid.is_valid());

  // Invalid font size (too large)
  let invalid2 = Danmaku {
    content: "Test".to_string(),
    font_size: 40,
    color: 0x00FF_FFFF,
    position: DanmakuPosition::Scroll,
    video_time_ms: 0,
    timestamp_nanos: 0,
  };
  assert!(!invalid2.is_valid());
}

// =============================================================================
// Comprehensive Discriminator Tests — verify every variant
// =============================================================================

#[test]
fn test_discriminator_chat_messages() {
  use super::discriminator;

  assert_eq!(
    DataChannelMessage::ChatText(ChatText {
      message_id: MessageId::new(),
      content: String::new(),
      reply_to: None,
      timestamp_nanos: 0,
    })
    .discriminator(),
    discriminator::CHAT_TEXT
  );

  assert_eq!(
    DataChannelMessage::ChatSticker(ChatSticker {
      message_id: MessageId::new(),
      pack_id: String::new(),
      sticker_id: String::new(),
      reply_to: None,
      timestamp_nanos: 0,
    })
    .discriminator(),
    discriminator::CHAT_STICKER
  );

  assert_eq!(
    DataChannelMessage::ChatVoice(ChatVoice {
      message_id: MessageId::new(),
      audio_data: vec![],
      duration_ms: 0,
      waveform: vec![],
      reply_to: None,
      timestamp_nanos: 0,
    })
    .discriminator(),
    discriminator::CHAT_VOICE
  );

  assert_eq!(
    DataChannelMessage::ChatImage(ChatImage {
      message_id: MessageId::new(),
      image_data: vec![],
      thumbnail: vec![],
      width: 0,
      height: 0,
      reply_to: None,
      timestamp_nanos: 0,
    })
    .discriminator(),
    discriminator::CHAT_IMAGE
  );
}

#[test]
fn test_discriminator_file_transfer() {
  use super::discriminator;

  assert_eq!(
    DataChannelMessage::FileChunk(FileChunk {
      transfer_id: TransferId::new(),
      chunk_index: 0,
      total_chunks: 0,
      data: vec![],
      chunk_hash: [0u8; 32],
    })
    .discriminator(),
    discriminator::FILE_CHUNK
  );

  assert_eq!(
    DataChannelMessage::FileMetadata(FileMetadata {
      message_id: MessageId::new(),
      transfer_id: TransferId::new(),
      filename: String::new(),
      size: 0,
      mime_type: String::new(),
      file_hash: [0u8; 32],
      total_chunks: 0,
      chunk_size: 0,
      reply_to: None,
      timestamp_nanos: 0,
    })
    .discriminator(),
    discriminator::FILE_METADATA
  );
}

#[test]
fn test_discriminator_message_control() {
  use super::discriminator;

  assert_eq!(
    DataChannelMessage::MessageAck(MessageAck {
      message_id: MessageId::new(),
      status: AckStatus::Received,
      timestamp_nanos: 0,
    })
    .discriminator(),
    discriminator::MESSAGE_ACK
  );

  assert_eq!(
    DataChannelMessage::MessageRevoke(MessageRevoke {
      message_id: MessageId::new(),
      timestamp_nanos: 0,
    })
    .discriminator(),
    discriminator::MESSAGE_REVOKE
  );

  assert_eq!(
    DataChannelMessage::TypingIndicator(TypingIndicator { is_typing: false }).discriminator(),
    discriminator::TYPING_INDICATOR
  );

  assert_eq!(
    DataChannelMessage::MessageRead(MessageRead {
      message_ids: vec![],
      timestamp_nanos: 0,
    })
    .discriminator(),
    discriminator::MESSAGE_READ
  );
}

#[test]
fn test_discriminator_message_enhancement() {
  use super::discriminator;

  assert_eq!(
    DataChannelMessage::ForwardMessage(ForwardMessage {
      message_id: MessageId::new(),
      original_message_id: MessageId::new(),
      original_sender: UserId::new(),
      content: String::new(),
      timestamp_nanos: 0,
    })
    .discriminator(),
    discriminator::FORWARD_MESSAGE
  );

  assert_eq!(
    DataChannelMessage::MessageReaction(MessageReaction {
      message_id: MessageId::new(),
      emoji: String::new(),
      action: ReactionAction::Add,
      timestamp_nanos: 0,
    })
    .discriminator(),
    discriminator::MESSAGE_REACTION
  );
}

#[test]
fn test_discriminator_encryption_and_avatar() {
  use super::discriminator;

  assert_eq!(
    DataChannelMessage::EcdhKeyExchange(EcdhKeyExchange {
      public_key: vec![0u8; 65],
      timestamp_nanos: 0,
    })
    .discriminator(),
    discriminator::ECDH_KEY_EXCHANGE
  );

  assert_eq!(
    DataChannelMessage::AvatarRequest(AvatarRequest {
      user_id: UserId::new()
    })
    .discriminator(),
    discriminator::AVATAR_REQUEST
  );

  assert_eq!(
    DataChannelMessage::AvatarData(AvatarData {
      user_id: UserId::new(),
      data: vec![],
      mime_type: String::new(),
      width: 0,
      height: 0,
    })
    .discriminator(),
    discriminator::AVATAR_DATA
  );
}

#[test]
fn test_discriminator_theater() {
  use super::discriminator;

  assert_eq!(
    DataChannelMessage::Danmaku(Danmaku {
      content: String::new(),
      font_size: 24,
      color: 0,
      position: DanmakuPosition::Scroll,
      video_time_ms: 0,
      timestamp_nanos: 0,
    })
    .discriminator(),
    discriminator::DANMAKU
  );

  assert_eq!(
    DataChannelMessage::PlaybackProgress(PlaybackProgress {
      room_id: RoomId::new(),
      current_time_ms: 0,
      duration_ms: 0,
      is_paused: false,
      timestamp_nanos: 0,
    })
    .discriminator(),
    discriminator::PLAYBACK_PROGRESS
  );

  assert_eq!(
    DataChannelMessage::SubtitleData(SubtitleData {
      room_id: RoomId::new(),
      entries: vec![],
    })
    .discriminator(),
    discriminator::SUBTITLE_DATA
  );

  assert_eq!(
    DataChannelMessage::SubtitleClear(SubtitleClear {
      room_id: RoomId::new()
    })
    .discriminator(),
    discriminator::SUBTITLE_CLEAR
  );
}

/// Create discriminators for chat message `DataChannelMessage` variants.
fn create_chat_message_discriminators() -> Vec<u8> {
  vec![
    DataChannelMessage::ChatText(ChatText {
      message_id: MessageId::new(),
      content: String::new(),
      reply_to: None,
      timestamp_nanos: 0,
    })
    .discriminator(),
    DataChannelMessage::ChatSticker(ChatSticker {
      message_id: MessageId::new(),
      pack_id: String::new(),
      sticker_id: String::new(),
      reply_to: None,
      timestamp_nanos: 0,
    })
    .discriminator(),
    DataChannelMessage::ChatVoice(ChatVoice {
      message_id: MessageId::new(),
      audio_data: vec![],
      duration_ms: 0,
      waveform: vec![],
      reply_to: None,
      timestamp_nanos: 0,
    })
    .discriminator(),
    DataChannelMessage::ChatImage(ChatImage {
      message_id: MessageId::new(),
      image_data: vec![],
      thumbnail: vec![],
      width: 0,
      height: 0,
      reply_to: None,
      timestamp_nanos: 0,
    })
    .discriminator(),
  ]
}

/// Create discriminators for file transfer `DataChannelMessage` variants.
fn create_file_transfer_discriminators() -> Vec<u8> {
  vec![
    DataChannelMessage::FileChunk(FileChunk {
      transfer_id: TransferId::new(),
      chunk_index: 0,
      total_chunks: 0,
      data: vec![],
      chunk_hash: [0u8; 32],
    })
    .discriminator(),
    DataChannelMessage::FileMetadata(FileMetadata {
      message_id: MessageId::new(),
      transfer_id: TransferId::new(),
      filename: String::new(),
      size: 0,
      mime_type: String::new(),
      file_hash: [0u8; 32],
      total_chunks: 0,
      chunk_size: 0,
      reply_to: None,
      timestamp_nanos: 0,
    })
    .discriminator(),
  ]
}

/// Create discriminators for message status `DataChannelMessage` variants.
fn create_message_status_discriminators() -> Vec<u8> {
  vec![
    DataChannelMessage::MessageAck(MessageAck {
      message_id: MessageId::new(),
      status: AckStatus::Received,
      timestamp_nanos: 0,
    })
    .discriminator(),
    DataChannelMessage::MessageRevoke(MessageRevoke {
      message_id: MessageId::new(),
      timestamp_nanos: 0,
    })
    .discriminator(),
    DataChannelMessage::TypingIndicator(TypingIndicator { is_typing: false }).discriminator(),
    DataChannelMessage::MessageRead(MessageRead {
      message_ids: vec![],
      timestamp_nanos: 0,
    })
    .discriminator(),
    DataChannelMessage::ForwardMessage(ForwardMessage {
      message_id: MessageId::new(),
      original_message_id: MessageId::new(),
      original_sender: UserId::new(),
      content: String::new(),
      timestamp_nanos: 0,
    })
    .discriminator(),
    DataChannelMessage::MessageReaction(MessageReaction {
      message_id: MessageId::new(),
      emoji: String::new(),
      action: ReactionAction::Add,
      timestamp_nanos: 0,
    })
    .discriminator(),
  ]
}

/// Create discriminators for user/avatar `DataChannelMessage` variants.
fn create_user_avatar_discriminators() -> Vec<u8> {
  vec![
    DataChannelMessage::EcdhKeyExchange(EcdhKeyExchange {
      public_key: vec![0u8; 65],
      timestamp_nanos: 0,
    })
    .discriminator(),
    DataChannelMessage::AvatarRequest(AvatarRequest {
      user_id: UserId::new(),
    })
    .discriminator(),
    DataChannelMessage::AvatarData(AvatarData {
      user_id: UserId::new(),
      data: vec![],
      mime_type: String::new(),
      width: 0,
      height: 0,
    })
    .discriminator(),
  ]
}

/// Create discriminators for theater `DataChannelMessage` variants.
fn create_theater_discriminators() -> Vec<u8> {
  vec![
    DataChannelMessage::Danmaku(Danmaku {
      content: String::new(),
      font_size: 24,
      color: 0,
      position: DanmakuPosition::Scroll,
      video_time_ms: 0,
      timestamp_nanos: 0,
    })
    .discriminator(),
    DataChannelMessage::PlaybackProgress(PlaybackProgress {
      room_id: RoomId::new(),
      current_time_ms: 0,
      duration_ms: 0,
      is_paused: false,
      timestamp_nanos: 0,
    })
    .discriminator(),
    DataChannelMessage::SubtitleData(SubtitleData {
      room_id: RoomId::new(),
      entries: vec![],
    })
    .discriminator(),
    DataChannelMessage::SubtitleClear(SubtitleClear {
      room_id: RoomId::new(),
    })
    .discriminator(),
  ]
}

/// Create discriminators for all `DataChannelMessage` variants.
fn create_all_discriminators() -> Vec<u8> {
  let mut discriminators = Vec::new();
  discriminators.extend(create_chat_message_discriminators());
  discriminators.extend(create_file_transfer_discriminators());
  discriminators.extend(create_message_status_discriminators());
  discriminators.extend(create_user_avatar_discriminators());
  discriminators.extend(create_theater_discriminators());
  discriminators
}

#[test]
fn test_discriminator_all_values_unique() {
  let discriminators = create_all_discriminators();

  let mut seen = std::collections::HashSet::new();
  for d in &discriminators {
    assert!(seen.insert(*d), "Duplicate discriminator value: {d:#04x}");
  }
  assert_eq!(
    discriminators.len(),
    19,
    "Should have 19 DataChannel variants"
  );
}

// =============================================================================
// Danmaku Validation Boundary Tests (P2-2)
// =============================================================================

#[test]
fn test_danmaku_is_valid_empty_content() {
  let danmaku = Danmaku {
    content: String::new(),
    font_size: 24,
    color: 0xFF_FF_FF,
    position: DanmakuPosition::Scroll,
    video_time_ms: 0,
    timestamp_nanos: 0,
  };
  assert!(!danmaku.is_valid(), "Empty content should be invalid");
}

#[test]
fn test_danmaku_is_valid_max_content_length() {
  // Content exactly at MAX_CONTENT_LENGTH (100 chars) should be valid
  let content = "A".repeat(Danmaku::MAX_CONTENT_LENGTH);
  let danmaku = Danmaku {
    content,
    font_size: 24,
    color: 0xFF_FF_FF,
    position: DanmakuPosition::Scroll,
    video_time_ms: 0,
    timestamp_nanos: 0,
  };
  assert!(danmaku.is_valid(), "Content at max length should be valid");
}

#[test]
fn test_danmaku_is_valid_exceeds_max_content_length() {
  // Content exceeding MAX_CONTENT_LENGTH should be invalid
  let content = "A".repeat(Danmaku::MAX_CONTENT_LENGTH + 1);
  let danmaku = Danmaku {
    content,
    font_size: 24,
    color: 0xFF_FF_FF,
    position: DanmakuPosition::Scroll,
    video_time_ms: 0,
    timestamp_nanos: 0,
  };
  assert!(
    !danmaku.is_valid(),
    "Content exceeding max length should be invalid"
  );
}

#[test]
fn test_danmaku_is_valid_min_font_size() {
  let danmaku = Danmaku {
    content: "Test".to_string(),
    font_size: Danmaku::MIN_FONT_SIZE,
    color: 0xFF_FF_FF,
    position: DanmakuPosition::Scroll,
    video_time_ms: 0,
    timestamp_nanos: 0,
  };
  assert!(danmaku.is_valid(), "Min font size should be valid");
}

#[test]
fn test_danmaku_is_valid_below_min_font_size() {
  let danmaku = Danmaku {
    content: "Test".to_string(),
    font_size: Danmaku::MIN_FONT_SIZE - 1,
    color: 0xFF_FF_FF,
    position: DanmakuPosition::Scroll,
    video_time_ms: 0,
    timestamp_nanos: 0,
  };
  assert!(!danmaku.is_valid(), "Below min font size should be invalid");
}

#[test]
fn test_danmaku_is_valid_max_font_size() {
  let danmaku = Danmaku {
    content: "Test".to_string(),
    font_size: Danmaku::MAX_FONT_SIZE,
    color: 0xFF_FF_FF,
    position: DanmakuPosition::Scroll,
    video_time_ms: 0,
    timestamp_nanos: 0,
  };
  assert!(danmaku.is_valid(), "Max font size should be valid");
}

#[test]
fn test_danmaku_is_valid_above_max_font_size() {
  let danmaku = Danmaku {
    content: "Test".to_string(),
    font_size: Danmaku::MAX_FONT_SIZE + 1,
    color: 0xFF_FF_FF,
    position: DanmakuPosition::Scroll,
    video_time_ms: 0,
    timestamp_nanos: 0,
  };
  assert!(!danmaku.is_valid(), "Above max font size should be invalid");
}

#[test]
fn test_danmaku_is_valid_all_positions() {
  for position in [
    DanmakuPosition::Scroll,
    DanmakuPosition::Top,
    DanmakuPosition::Bottom,
  ] {
    let danmaku = Danmaku {
      content: "Test".to_string(),
      font_size: 24,
      color: 0xFF_FF_FF,
      position,
      video_time_ms: 0,
      timestamp_nanos: 0,
    };
    assert!(danmaku.is_valid(), "Position {position:?} should be valid");
  }
}

// =============================================================================
// SubtitleEntry Boundary Tests (P2-2)
// =============================================================================

#[test]
fn test_subtitle_entry_is_active_at_exact_start() {
  let entry = SubtitleEntry::new(1000, 3000, "Test".to_string());
  assert!(
    entry.is_active_at(1000),
    "Should be active at exact start_ms"
  );
}

#[test]
fn test_subtitle_entry_is_active_at_just_before_end() {
  let entry = SubtitleEntry::new(1000, 3000, "Test".to_string());
  assert!(
    entry.is_active_at(2999),
    "Should be active just before end_ms"
  );
}

#[test]
fn test_subtitle_entry_not_active_at_exact_end() {
  let entry = SubtitleEntry::new(1000, 3000, "Test".to_string());
  assert!(
    !entry.is_active_at(3000),
    "Should not be active at exact end_ms (exclusive)"
  );
}

#[test]
fn test_subtitle_entry_not_active_just_before_start() {
  let entry = SubtitleEntry::new(1000, 3000, "Test".to_string());
  assert!(
    !entry.is_active_at(999),
    "Should not be active just before start_ms"
  );
}

#[test]
fn test_subtitle_entry_zero_duration() {
  let entry = SubtitleEntry::new(1000, 1000, "Instant".to_string());
  assert_eq!(entry.duration_ms(), 0, "Zero duration should return 0");
  assert!(
    !entry.is_active_at(1000),
    "Zero duration entry should not be active at start time"
  );
  assert!(
    !entry.is_active_at(999),
    "Zero duration entry should not be active before start"
  );
}

// =============================================================================
// AckStatus Boundary Tests (P2-2)
// =============================================================================

#[test]
fn test_ack_status_failed_roundtrip() {
  let msg = MessageAck {
    message_id: MessageId::new(),
    status: AckStatus::Failed,
    timestamp_nanos: 1_000_000_000,
  };
  test_bitcode_roundtrip(&msg);
}

#[test]
fn test_ack_status_both_variants_roundtrip() {
  for status in [AckStatus::Received, AckStatus::Failed] {
    let msg = MessageAck {
      message_id: MessageId::new(),
      status,
      timestamp_nanos: 1_000_000_000,
    };
    test_bitcode_roundtrip(&msg);
  }
}

#[test]
fn test_ack_status_json_roundtrip() {
  for status in [AckStatus::Received, AckStatus::Failed] {
    let json = serde_json::to_string(&status).expect("Failed to serialize AckStatus");
    let decoded: AckStatus = serde_json::from_str(&json).expect("Failed to deserialize AckStatus");
    assert_eq!(status, decoded);
  }
}

// =============================================================================
// FileChunk Boundary Tests (P2-2)
// =============================================================================

#[test]
fn test_file_chunk_last_chunk() {
  let msg = FileChunk {
    transfer_id: TransferId::new(),
    chunk_index: 9,
    total_chunks: 10,
    data: vec![0xAB; 1024],
    chunk_hash: [0u8; 32],
  };
  test_bitcode_roundtrip(&msg);
}

#[test]
fn test_file_chunk_single_chunk() {
  let msg = FileChunk {
    transfer_id: TransferId::new(),
    chunk_index: 0,
    total_chunks: 1,
    data: vec![0xCD; 512],
    chunk_hash: [0u8; 32],
  };
  test_bitcode_roundtrip(&msg);
}

#[test]
fn test_file_chunk_empty_data() {
  let msg = FileChunk {
    transfer_id: TransferId::new(),
    chunk_index: 0,
    total_chunks: 5,
    data: vec![],
    chunk_hash: [0u8; 32],
  };
  test_bitcode_roundtrip(&msg);
}

#[test]
fn test_file_chunk_max_size_data() {
  // Max chunk data size is 64KB
  let msg = FileChunk {
    transfer_id: TransferId::new(),
    chunk_index: 0,
    total_chunks: 10,
    data: vec![0xFF; 65536],
    chunk_hash: [0u8; 32],
  };
  test_bitcode_roundtrip(&msg);
}

#[test]
fn test_file_metadata_zero_size() {
  let msg = FileMetadata {
    message_id: MessageId::new(),
    transfer_id: TransferId::new(),
    filename: "empty.txt".to_string(),
    size: 0,
    mime_type: "text/plain".to_string(),
    file_hash: [0u8; 32],
    total_chunks: 0,
    chunk_size: 0,
    reply_to: None,
    timestamp_nanos: 0,
  };
  test_bitcode_roundtrip(&msg);
}

// =============================================================================
// ReactionAction::Remove Roundtrip (P2-2)
// =============================================================================

#[test]
fn test_message_reaction_remove_roundtrip() {
  let msg = MessageReaction {
    message_id: MessageId::new(),
    emoji: "❤️".to_string(),
    action: ReactionAction::Remove,
    timestamp_nanos: 1_000_000_000,
  };
  test_bitcode_roundtrip(&msg);
}

#[test]
fn test_reaction_action_both_variants_roundtrip() {
  for action in [ReactionAction::Add, ReactionAction::Remove] {
    let msg = MessageReaction {
      message_id: MessageId::new(),
      emoji: "👍".to_string(),
      action,
      timestamp_nanos: 1_000_000_000,
    };
    test_bitcode_roundtrip(&msg);
  }
}

// =============================================================================
// DataChannelMessage JSON Serialization (P2-2)
// =============================================================================

#[test]
fn test_datachannel_message_json_roundtrip() {
  let msg = DataChannelMessage::ChatText(ChatText {
    message_id: MessageId::new(),
    content: "Hello, **world**!".to_string(),
    reply_to: Some(MessageId::new()),
    timestamp_nanos: 1_000_000_000,
  });
  let json = serde_json::to_string(&msg).expect("Failed to serialize DataChannelMessage");
  let decoded: DataChannelMessage =
    serde_json::from_str(&json).expect("Failed to deserialize DataChannelMessage");
  assert_eq!(msg, decoded);
}

#[test]
fn test_datachannel_message_json_roundtrip_all_variants() {
  let messages = vec![
    DataChannelMessage::ChatSticker(ChatSticker {
      message_id: MessageId::new(),
      pack_id: "pack_001".to_string(),
      sticker_id: "sticker_123".to_string(),
      reply_to: None,
      timestamp_nanos: 1_000_000_000,
    }),
    DataChannelMessage::MessageAck(MessageAck {
      message_id: MessageId::new(),
      status: AckStatus::Failed,
      timestamp_nanos: 1_000_000_000,
    }),
    DataChannelMessage::TypingIndicator(TypingIndicator { is_typing: true }),
    DataChannelMessage::Danmaku(Danmaku {
      content: "Bullet comment".to_string(),
      font_size: 24,
      color: 0xFF_FF_FF,
      position: DanmakuPosition::Top,
      video_time_ms: 5000,
      timestamp_nanos: 1_000_000_000,
    }),
    DataChannelMessage::PlaybackProgress(PlaybackProgress {
      room_id: RoomId::new(),
      current_time_ms: 30000,
      duration_ms: 3_600_000,
      is_paused: true,
      timestamp_nanos: 1_000_000_000,
    }),
    DataChannelMessage::SubtitleClear(SubtitleClear {
      room_id: RoomId::new(),
    }),
  ];

  for msg in messages {
    let json = serde_json::to_string(&msg).expect("Failed to serialize DataChannelMessage");
    let decoded: DataChannelMessage =
      serde_json::from_str(&json).expect("Failed to deserialize DataChannelMessage");
    assert_eq!(msg, decoded, "JSON roundtrip failed for variant");
  }
}
