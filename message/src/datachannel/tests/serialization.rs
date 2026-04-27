use super::*;

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
