use super::*;

// =============================================================================
// Danmaku Validation Boundary Tests (P2-2)
// =============================================================================

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
