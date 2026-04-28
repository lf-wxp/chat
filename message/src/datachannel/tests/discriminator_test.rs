use super::*;

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
      room_id: None,
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
fn test_discriminator_chat_messages() {
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
      room_id: None,
    })
    .discriminator(),
    discriminator::FILE_METADATA
  );
}

#[test]
fn test_discriminator_message_control() {
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
      user_id: UserId::new(),
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
      room_id: RoomId::new(),
    })
    .discriminator(),
    discriminator::SUBTITLE_CLEAR
  );
}

// --- Helper functions for `test_discriminator_all_values_unique` ---

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
      room_id: None,
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

#[test]
fn test_discriminator_all_values_unique() {
  let mut discriminators = Vec::new();
  discriminators.extend(create_chat_message_discriminators());
  discriminators.extend(create_file_transfer_discriminators());
  discriminators.extend(create_message_status_discriminators());
  discriminators.extend(create_user_avatar_discriminators());
  discriminators.extend(create_theater_discriminators());

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

#[test]
fn test_media_state_update_discriminator() {
  let msg = DataChannelMessage::MediaStateUpdate(MediaStateUpdate {
    mic_enabled: false,
    camera_enabled: false,
    screen_sharing: false,
  });
  assert_eq!(msg.discriminator(), discriminator::MEDIA_STATE_UPDATE);
  assert_eq!(msg.discriminator(), 0xC0);
}

#[test]
fn test_reconnecting_state_discriminator() {
  let msg = DataChannelMessage::ReconnectingState(ReconnectingState { reconnecting: true });
  assert_eq!(msg.discriminator(), discriminator::RECONNECTING_STATE);
  assert_eq!(msg.discriminator(), 0xC1);
}

#[test]
fn test_call_status_messages_are_lightweight() {
  // Both call-status broadcasts should be lightweight so they fit
  // through the ACK queue's JSON-persistence fast path (Req 11.3).
  let media = DataChannelMessage::MediaStateUpdate(MediaStateUpdate {
    mic_enabled: true,
    camera_enabled: true,
    screen_sharing: false,
  });
  let recon = DataChannelMessage::ReconnectingState(ReconnectingState {
    reconnecting: false,
  });
  assert!(media.is_lightweight());
  assert!(recon.is_lightweight());
}
