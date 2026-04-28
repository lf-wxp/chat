use super::*;

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
    room_id: None,
  };
  test_bitcode_roundtrip(&msg);
}

#[test]
fn test_file_resume_request_roundtrip() {
  let msg = FileResumeRequest {
    transfer_id: TransferId::new(),
    missing_chunks: vec![0, 3, 7, 15],
    timestamp_nanos: 1_000_000_000,
  };
  test_bitcode_roundtrip(&msg);

  // Empty missing_chunks (keep-alive / ack).
  let empty = FileResumeRequest {
    transfer_id: TransferId::new(),
    missing_chunks: vec![],
    timestamp_nanos: 1_000_000_000,
  };
  test_bitcode_roundtrip(&empty);

  // Full retransmit request.
  let full = FileResumeRequest {
    transfer_id: TransferId::new(),
    missing_chunks: (0..100).collect(),
    timestamp_nanos: 1_000_000_000,
  };
  test_bitcode_roundtrip(&full);
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
fn test_media_state_update_roundtrip() {
  let msg = MediaStateUpdate {
    mic_enabled: true,
    camera_enabled: false,
    screen_sharing: true,
  };
  test_bitcode_roundtrip(&msg);
}

#[test]
fn test_reconnecting_state_roundtrip() {
  let on = ReconnectingState { reconnecting: true };
  let off = ReconnectingState {
    reconnecting: false,
  };
  test_bitcode_roundtrip(&on);
  test_bitcode_roundtrip(&off);
}
