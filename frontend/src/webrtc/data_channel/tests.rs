use message::datachannel::AckStatus;
use message::datachannel::{
  AvatarData, AvatarRequest, ChatImage, ChatSticker, ChatText, ChatVoice, Danmaku, DanmakuBatch,
  DataChannelMessage, EcdhKeyExchange, FileChunk, FileMetadata, FileResumeRequest, ForwardMessage,
  MediaStateUpdate, MessageAck, MessageReaction, MessageRead, MessageRevoke, PlaybackProgress,
  ReactionAction, ReconnectingState, SubtitleClear, SubtitleData, SubtitleEntry, TheaterChatText,
  TypingIndicator,
};

fn uid() -> message::UserId {
  message::UserId::from_uuid(uuid::Uuid::new_v4())
}

fn mid() -> message::MessageId {
  message::MessageId(uuid::Uuid::new_v4())
}

fn tid() -> message::TransferId {
  message::TransferId(uuid::Uuid::new_v4())
}

fn rid() -> message::RoomId {
  message::RoomId::from_uuid(uuid::Uuid::new_v4())
}

// ---------------------------------------------------------------------------
// Discriminator uniqueness
// ---------------------------------------------------------------------------

#[test]
fn all_discriminators_are_unique() {
  let msgs: Vec<DataChannelMessage> = vec![
    DataChannelMessage::ChatText(ChatText {
      message_id: mid(),
      content: "t".to_string(),
      reply_to: None,
      timestamp_nanos: 0,
      room_id: None,
      mentions: vec![],
    }),
    DataChannelMessage::ChatSticker(ChatSticker {
      message_id: mid(),
      pack_id: "p".to_string(),
      sticker_id: "s".to_string(),
      reply_to: None,
      timestamp_nanos: 0,
      room_id: None,
    }),
    DataChannelMessage::ChatVoice(ChatVoice {
      message_id: mid(),
      audio_data: vec![0u8; 64],
      duration_ms: 1000,
      waveform: vec![0u8; 16],
      reply_to: None,
      timestamp_nanos: 0,
      room_id: None,
    }),
    DataChannelMessage::ChatImage(ChatImage {
      message_id: mid(),
      image_data: vec![0u8; 32],
      thumbnail: vec![0u8; 8],
      width: 100,
      height: 100,
      reply_to: None,
      timestamp_nanos: 0,
      room_id: None,
    }),
    DataChannelMessage::FileChunk(FileChunk {
      transfer_id: tid(),
      chunk_index: 0,
      total_chunks: 1,
      data: vec![1, 2, 3],
      chunk_hash: [0u8; 32],
    }),
    DataChannelMessage::FileMetadata(FileMetadata {
      message_id: mid(),
      transfer_id: tid(),
      filename: "a.txt".to_string(),
      size: 42,
      mime_type: "text/plain".to_string(),
      file_hash: [0u8; 32],
      total_chunks: 1,
      chunk_size: 65536,
      reply_to: None,
      timestamp_nanos: 0,
      room_id: None,
    }),
    DataChannelMessage::FileResumeRequest(FileResumeRequest {
      transfer_id: tid(),
      missing_chunks: vec![5],
      timestamp_nanos: 0,
    }),
    DataChannelMessage::MessageAck(MessageAck {
      message_id: mid(),
      status: AckStatus::Received,
      timestamp_nanos: 0,
    }),
    DataChannelMessage::MessageRevoke(MessageRevoke {
      message_id: mid(),
      timestamp_nanos: 0,
    }),
    DataChannelMessage::TypingIndicator(TypingIndicator { is_typing: true }),
    DataChannelMessage::MessageRead(MessageRead {
      message_ids: vec![mid()],
      timestamp_nanos: 0,
    }),
    DataChannelMessage::ForwardMessage(ForwardMessage {
      message_id: mid(),
      original_message_id: mid(),
      original_sender: uid(),
      content: "fwd".to_string(),
      timestamp_nanos: 0,
      room_id: None,
    }),
    DataChannelMessage::MessageReaction(MessageReaction {
      message_id: mid(),
      emoji: "👍".to_string(),
      action: ReactionAction::Add,
      timestamp_nanos: 0,
    }),
    DataChannelMessage::EcdhKeyExchange(EcdhKeyExchange {
      public_key: vec![0u8; 65],
      timestamp_nanos: 0,
    }),
    DataChannelMessage::AvatarRequest(AvatarRequest { user_id: uid() }),
    DataChannelMessage::AvatarData(AvatarData {
      user_id: uid(),
      data: vec![0u8; 10],
      mime_type: "image/png".to_string(),
      width: 64,
      height: 64,
    }),
    DataChannelMessage::Danmaku(Danmaku {
      content: "hi".to_string(),
      font_size: 24,
      color: 0xFFFFFF,
      position: message::types::DanmakuPosition::Scroll,
      video_time_ms: 0,
      timestamp_nanos: 0,
    }),
    DataChannelMessage::PlaybackProgress(PlaybackProgress {
      room_id: rid(),
      current_time_ms: 5000,
      duration_ms: 30000,
      is_paused: false,
      timestamp_nanos: 0,
    }),
    DataChannelMessage::SubtitleData(SubtitleData {
      room_id: rid(),
      entries: vec![SubtitleEntry {
        start_ms: 0,
        end_ms: 2000,
        text: "hello".to_string(),
      }],
    }),
    DataChannelMessage::SubtitleClear(SubtitleClear { room_id: rid() }),
    DataChannelMessage::DanmakuBatch(DanmakuBatch {
      room_id: rid(),
      entries: vec![],
    }),
    DataChannelMessage::TheaterChatText(TheaterChatText {
      room_id: rid(),
      sender_id: uid(),
      content: "tc".to_string(),
      timestamp_nanos: 0,
    }),
    DataChannelMessage::MediaStateUpdate(MediaStateUpdate {
      mic_enabled: true,
      camera_enabled: false,
      screen_sharing: false,
    }),
    DataChannelMessage::ReconnectingState(ReconnectingState { reconnecting: true }),
  ];

  let discriminators: Vec<u8> = msgs.iter().map(|m| m.discriminator()).collect();
  let mut unique = discriminators.clone();
  unique.sort();
  unique.dedup();
  assert_eq!(
    discriminators.len(),
    unique.len(),
    "Duplicate discriminator detected among {:?}",
    discriminators
  );
}

#[test]
fn all_discriminators_are_in_valid_range() {
  let msgs = all_message_variants();
  for msg in &msgs {
    let d = msg.discriminator();
    assert!(
      (0x80..=0xC3).contains(&d) || d == 0xFE,
      "Discriminator 0x{d:02X} outside valid range"
    );
  }
}

// ---------------------------------------------------------------------------
// Frame layout
// ---------------------------------------------------------------------------

#[test]
fn message_discriminator_roundtrip() {
  let msg = DataChannelMessage::ChatText(ChatText {
    message_id: mid(),
    content: "Hello, World!".to_string(),
    reply_to: None,
    timestamp_nanos: 1234567890,
    room_id: None,
    mentions: vec![],
  });

  let discriminator = msg.discriminator();
  let payload = bitcode::encode(&msg);

  let mut frame = Vec::new();
  frame.push(discriminator);
  frame.extend_from_slice(&payload);

  let decoded_discriminator = frame[0];
  let decoded_payload = &frame[1..];
  let decoded: DataChannelMessage = bitcode::decode(decoded_payload).unwrap();

  assert_eq!(discriminator, decoded_discriminator);
  assert_eq!(msg.discriminator(), decoded.discriminator());

  if let DataChannelMessage::ChatText(ref ct) = decoded {
    assert_eq!(ct.content, "Hello, World!");
  } else {
    panic!("Expected ChatText message");
  }
}

/// Task 19.1 — the encrypted-envelope marker byte must live strictly
/// outside the range used by `DataChannelMessage::discriminator()` so
/// the receive path can route plaintext vs envelope frames on the
/// first byte alone.
#[test]
fn encrypted_marker_is_outside_discriminator_range() {
  use super::ENCRYPTED_MARKER;

  let disc_range = 0x80..=0xC3;
  assert!(
    !disc_range.contains(&ENCRYPTED_MARKER),
    "ENCRYPTED_MARKER (0x{:02X}) must not collide with any DataChannelMessage discriminator",
    ENCRYPTED_MARKER
  );
  assert_eq!(
    ENCRYPTED_MARKER, 0xFE,
    "The envelope marker is part of the on-the-wire protocol and must remain stable"
  );
}

/// Task 19.1 — synthesise an envelope frame and parse it back,
/// ensuring the layout `[ENCRYPTED_MARKER][iv (12 B)][ciphertext]`
/// round-trips through the byte-level split that
/// `WebRtcManager::handle_data_channel_raw_frame` performs.
#[test]
fn encrypted_envelope_frame_layout_round_trips() {
  use super::ENCRYPTED_MARKER;

  let iv = [0x11u8; 12];
  let ciphertext = [0x22u8; 20];

  let mut frame = Vec::with_capacity(1 + iv.len() + ciphertext.len());
  frame.push(ENCRYPTED_MARKER);
  frame.extend_from_slice(&iv);
  frame.extend_from_slice(&ciphertext);

  assert_eq!(frame[0], ENCRYPTED_MARKER);
  let body = &frame[1..];
  let (iv_part, ct_part) = body.split_at(12);
  assert_eq!(iv_part, &iv);
  assert_eq!(ct_part, &ciphertext);
}

#[test]
fn encrypted_envelope_empty_ciphertext() {
  use super::ENCRYPTED_MARKER;

  // Edge case: ciphertext could be empty (e.g. zero-length payload)
  let iv = [0xAAu8; 12];
  let frame = {
    let mut f = Vec::with_capacity(1 + 12);
    f.push(ENCRYPTED_MARKER);
    f.extend_from_slice(&iv);
    f
  };

  assert_eq!(frame[0], ENCRYPTED_MARKER);
  let body = &frame[1..];
  assert_eq!(body.len(), 12);
  assert_eq!(body, &iv);
}

#[test]
fn encrypted_envelope_frame_minimum_size_is_13() {
  // Minimum valid envelope: [1 marker] + [12 IV] = 13 bytes
  // Even with empty ciphertext, the frame must be at least 13 bytes.
  use super::ENCRYPTED_MARKER;
  let min_frame = {
    let mut f = Vec::with_capacity(13);
    f.push(ENCRYPTED_MARKER);
    f.extend_from_slice(&[0u8; 12]);
    f
  };
  assert_eq!(min_frame.len(), 13);
}

// ---------------------------------------------------------------------------
// Multiple message type round-trips
// ---------------------------------------------------------------------------

#[test]
fn chat_sticker_roundtrip() {
  let msg = DataChannelMessage::ChatSticker(ChatSticker {
    message_id: mid(),
    pack_id: "animals".to_string(),
    sticker_id: "cat".to_string(),
    reply_to: None,
    timestamp_nanos: 0,
    room_id: None,
  });
  let payload = bitcode::encode(&msg);
  let decoded: DataChannelMessage = bitcode::decode(&payload).unwrap();
  if let DataChannelMessage::ChatSticker(s) = decoded {
    assert_eq!(s.pack_id, "animals");
    assert_eq!(s.sticker_id, "cat");
  } else {
    panic!("Expected ChatSticker");
  }
}

#[test]
fn ecdh_key_exchange_roundtrip() {
  let msg = DataChannelMessage::EcdhKeyExchange(EcdhKeyExchange {
    public_key: vec![0x04; 65],
    timestamp_nanos: 42,
  });
  let payload = bitcode::encode(&msg);
  let decoded: DataChannelMessage = bitcode::decode(&payload).unwrap();
  if let DataChannelMessage::EcdhKeyExchange(k) = decoded {
    assert_eq!(k.public_key.len(), 65);
    assert_eq!(k.public_key[0], 0x04);
    assert_eq!(k.timestamp_nanos, 42);
  } else {
    panic!("Expected EcdhKeyExchange");
  }
}

#[test]
fn typing_indicator_roundtrip() {
  let msg_on = DataChannelMessage::TypingIndicator(TypingIndicator { is_typing: true });
  let payload = bitcode::encode(&msg_on);
  let decoded: DataChannelMessage = bitcode::decode(&payload).unwrap();
  if let DataChannelMessage::TypingIndicator(t) = decoded {
    assert!(t.is_typing);
  } else {
    panic!("Expected TypingIndicator");
  }

  let msg_off = DataChannelMessage::TypingIndicator(TypingIndicator { is_typing: false });
  let payload = bitcode::encode(&msg_off);
  let decoded: DataChannelMessage = bitcode::decode(&payload).unwrap();
  if let DataChannelMessage::TypingIndicator(t) = decoded {
    assert!(!t.is_typing);
  } else {
    panic!("Expected TypingIndicator");
  }
}

#[test]
fn message_reaction_roundtrip() {
  let add = DataChannelMessage::MessageReaction(MessageReaction {
    message_id: mid(),
    emoji: "🎉".to_string(),
    action: ReactionAction::Add,
    timestamp_nanos: 0,
  });
  let decoded: DataChannelMessage = bitcode::decode(&bitcode::encode(&add)).unwrap();
  if let DataChannelMessage::MessageReaction(r) = decoded {
    assert_eq!(r.action, ReactionAction::Add);
    assert_eq!(r.emoji, "🎉");
  } else {
    panic!("Expected MessageReaction");
  }

  let remove = DataChannelMessage::MessageReaction(MessageReaction {
    message_id: mid(),
    emoji: "🎉".to_string(),
    action: ReactionAction::Remove,
    timestamp_nanos: 0,
  });
  let decoded: DataChannelMessage = bitcode::decode(&bitcode::encode(&remove)).unwrap();
  if let DataChannelMessage::MessageReaction(r) = decoded {
    assert_eq!(r.action, ReactionAction::Remove);
  } else {
    panic!("Expected MessageReaction");
  }
}

#[test]
fn media_state_update_roundtrip() {
  let msg = DataChannelMessage::MediaStateUpdate(MediaStateUpdate {
    mic_enabled: true,
    camera_enabled: false,
    screen_sharing: true,
  });
  let decoded: DataChannelMessage = bitcode::decode(&bitcode::encode(&msg)).unwrap();
  if let DataChannelMessage::MediaStateUpdate(m) = decoded {
    assert!(m.mic_enabled);
    assert!(!m.camera_enabled);
    assert!(m.screen_sharing);
  } else {
    panic!("Expected MediaStateUpdate");
  }
}

#[test]
fn file_chunk_roundtrip() {
  let msg = DataChannelMessage::FileChunk(FileChunk {
    transfer_id: tid(),
    chunk_index: 7,
    total_chunks: 10,
    data: vec![0xDE, 0xAD, 0xBE, 0xEF],
    chunk_hash: [0u8; 32],
  });
  let decoded: DataChannelMessage = bitcode::decode(&bitcode::encode(&msg)).unwrap();
  if let DataChannelMessage::FileChunk(c) = decoded {
    assert_eq!(c.chunk_index, 7);
    assert_eq!(c.total_chunks, 10);
    assert_eq!(c.data, vec![0xDE, 0xAD, 0xBE, 0xEF]);
  } else {
    panic!("Expected FileChunk");
  }
}

#[test]
fn danmaku_batch_roundtrip() {
  let msg = DataChannelMessage::DanmakuBatch(DanmakuBatch {
    room_id: rid(),
    entries: vec![
      Danmaku {
        content: "cool".to_string(),
        font_size: 24,
        color: 0xFFFF00,
        position: message::types::DanmakuPosition::Scroll,
        video_time_ms: 1000,
        timestamp_nanos: 0,
      },
      Danmaku {
        content: "nice".to_string(),
        font_size: 18,
        color: 0x00FFFF,
        position: message::types::DanmakuPosition::Top,
        video_time_ms: 2000,
        timestamp_nanos: 0,
      },
    ],
  });
  let decoded: DataChannelMessage = bitcode::decode(&bitcode::encode(&msg)).unwrap();
  if let DataChannelMessage::DanmakuBatch(b) = decoded {
    assert_eq!(b.entries.len(), 2);
    assert_eq!(b.entries[0].content, "cool");
    assert_eq!(b.entries[1].color, 0x00FFFF);
  } else {
    panic!("Expected DanmakuBatch");
  }
}

#[test]
fn subtitle_data_roundtrip() {
  let msg = DataChannelMessage::SubtitleData(SubtitleData {
    room_id: rid(),
    entries: vec![
      SubtitleEntry {
        start_ms: 0,
        end_ms: 2000,
        text: "Hello".to_string(),
      },
      SubtitleEntry {
        start_ms: 2000,
        end_ms: 4000,
        text: "World".to_string(),
      },
    ],
  });
  let decoded: DataChannelMessage = bitcode::decode(&bitcode::encode(&msg)).unwrap();
  if let DataChannelMessage::SubtitleData(s) = decoded {
    assert_eq!(s.entries.len(), 2);
    assert_eq!(s.entries[0].text, "Hello");
    assert_eq!(s.entries[1].start_ms, 2000);
  } else {
    panic!("Expected SubtitleData");
  }
}

#[test]
fn subtitle_clear_roundtrip() {
  let msg = DataChannelMessage::SubtitleClear(SubtitleClear { room_id: rid() });
  let decoded: DataChannelMessage = bitcode::decode(&bitcode::encode(&msg)).unwrap();
  if let DataChannelMessage::SubtitleClear(c) = decoded {
    // Ensure the room_id field deserializes correctly
    assert!(!c.room_id.as_uuid().is_nil());
  } else {
    panic!("Expected SubtitleClear");
  }
}

#[test]
fn theater_chat_text_roundtrip() {
  let msg = DataChannelMessage::TheaterChatText(TheaterChatText {
    room_id: rid(),
    sender_id: uid(),
    content: "theater msg".to_string(),
    timestamp_nanos: 999,
  });
  let decoded: DataChannelMessage = bitcode::decode(&bitcode::encode(&msg)).unwrap();
  if let DataChannelMessage::TheaterChatText(t) = decoded {
    assert_eq!(t.content, "theater msg");
    assert_eq!(t.timestamp_nanos, 999);
  } else {
    panic!("Expected TheaterChatText");
  }
}

#[test]
fn reconnecting_state_roundtrip() {
  let msg_on = DataChannelMessage::ReconnectingState(ReconnectingState { reconnecting: true });
  let decoded: DataChannelMessage = bitcode::decode(&bitcode::encode(&msg_on)).unwrap();
  if let DataChannelMessage::ReconnectingState(r) = decoded {
    assert!(r.reconnecting);
  } else {
    panic!("Expected ReconnectingState");
  }

  let msg_off = DataChannelMessage::ReconnectingState(ReconnectingState {
    reconnecting: false,
  });
  let decoded: DataChannelMessage = bitcode::decode(&bitcode::encode(&msg_off)).unwrap();
  if let DataChannelMessage::ReconnectingState(r) = decoded {
    assert!(!r.reconnecting);
  } else {
    panic!("Expected ReconnectingState");
  }
}

#[test]
fn message_ack_roundtrip() {
  let msg = DataChannelMessage::MessageAck(MessageAck {
    message_id: mid(),
    status: AckStatus::Received,
    timestamp_nanos: 100,
  });
  let decoded: DataChannelMessage = bitcode::decode(&bitcode::encode(&msg)).unwrap();
  if let DataChannelMessage::MessageAck(a) = decoded {
    assert_eq!(a.status, AckStatus::Received);
    assert_eq!(a.timestamp_nanos, 100);
  } else {
    panic!("Expected MessageAck");
  }

  let failed = DataChannelMessage::MessageAck(MessageAck {
    message_id: mid(),
    status: AckStatus::Failed,
    timestamp_nanos: 200,
  });
  let decoded: DataChannelMessage = bitcode::decode(&bitcode::encode(&failed)).unwrap();
  if let DataChannelMessage::MessageAck(a) = decoded {
    assert_eq!(a.status, AckStatus::Failed);
  } else {
    panic!("Expected MessageAck");
  }
}

#[test]
fn avatar_data_roundtrip() {
  let msg = DataChannelMessage::AvatarData(AvatarData {
    user_id: uid(),
    data: vec![0xFFu8; 256],
    mime_type: "image/jpeg".to_string(),
    width: 128,
    height: 128,
  });
  let decoded: DataChannelMessage = bitcode::decode(&bitcode::encode(&msg)).unwrap();
  if let DataChannelMessage::AvatarData(a) = decoded {
    assert_eq!(a.mime_type, "image/jpeg");
    assert_eq!(a.width, 128);
    assert_eq!(a.height, 128);
    assert_eq!(a.data.len(), 256);
  } else {
    panic!("Expected AvatarData");
  }
}

#[test]
fn forward_message_roundtrip() {
  let msg = DataChannelMessage::ForwardMessage(ForwardMessage {
    message_id: mid(),
    original_message_id: mid(),
    original_sender: uid(),
    content: "forwarded content".to_string(),
    timestamp_nanos: 12345,
    room_id: None,
  });
  let decoded: DataChannelMessage = bitcode::decode(&bitcode::encode(&msg)).unwrap();
  if let DataChannelMessage::ForwardMessage(f) = decoded {
    assert_eq!(f.content, "forwarded content");
    assert_eq!(f.timestamp_nanos, 12345);
  } else {
    panic!("Expected ForwardMessage");
  }
}

#[test]
fn playback_progress_roundtrip() {
  let msg = DataChannelMessage::PlaybackProgress(PlaybackProgress {
    room_id: rid(),
    current_time_ms: 5000,
    duration_ms: 120000,
    is_paused: true,
    timestamp_nanos: 0,
  });
  let decoded: DataChannelMessage = bitcode::decode(&bitcode::encode(&msg)).unwrap();
  if let DataChannelMessage::PlaybackProgress(p) = decoded {
    assert_eq!(p.current_time_ms, 5000);
    assert_eq!(p.duration_ms, 120000);
    assert!(p.is_paused);
  } else {
    panic!("Expected PlaybackProgress");
  }
}

#[test]
fn file_resume_request_roundtrip() {
  let msg = DataChannelMessage::FileResumeRequest(FileResumeRequest {
    transfer_id: tid(),
    missing_chunks: vec![3, 7, 11],
    timestamp_nanos: 0,
  });
  let decoded: DataChannelMessage = bitcode::decode(&bitcode::encode(&msg)).unwrap();
  if let DataChannelMessage::FileResumeRequest(r) = decoded {
    assert_eq!(r.missing_chunks, vec![3, 7, 11]);
  } else {
    panic!("Expected FileResumeRequest");
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create one instance of every `DataChannelMessage` variant.
fn all_message_variants() -> Vec<DataChannelMessage> {
  vec![
    DataChannelMessage::ChatText(ChatText {
      message_id: mid(),
      content: "t".to_string(),
      reply_to: None,
      timestamp_nanos: 0,
      room_id: None,
      mentions: vec![],
    }),
    DataChannelMessage::ChatSticker(ChatSticker {
      message_id: mid(),
      pack_id: "p".to_string(),
      sticker_id: "s".to_string(),
      reply_to: None,
      timestamp_nanos: 0,
      room_id: None,
    }),
    DataChannelMessage::ChatVoice(ChatVoice {
      message_id: mid(),
      audio_data: vec![0u8; 64],
      duration_ms: 1000,
      waveform: vec![0u8; 16],
      reply_to: None,
      timestamp_nanos: 0,
      room_id: None,
    }),
    DataChannelMessage::ChatImage(ChatImage {
      message_id: mid(),
      image_data: vec![0u8; 32],
      thumbnail: vec![0u8; 8],
      width: 100,
      height: 100,
      reply_to: None,
      timestamp_nanos: 0,
      room_id: None,
    }),
    DataChannelMessage::FileChunk(FileChunk {
      transfer_id: tid(),
      chunk_index: 0,
      total_chunks: 1,
      data: vec![1, 2, 3],
      chunk_hash: [0u8; 32],
    }),
    DataChannelMessage::FileMetadata(FileMetadata {
      message_id: mid(),
      transfer_id: tid(),
      filename: "a.txt".to_string(),
      size: 42,
      mime_type: "text/plain".to_string(),
      file_hash: [0u8; 32],
      total_chunks: 1,
      chunk_size: 65536,
      reply_to: None,
      timestamp_nanos: 0,
      room_id: None,
    }),
    DataChannelMessage::FileResumeRequest(FileResumeRequest {
      transfer_id: tid(),
      missing_chunks: vec![5],
      timestamp_nanos: 0,
    }),
    DataChannelMessage::MessageAck(MessageAck {
      message_id: mid(),
      status: AckStatus::Received,
      timestamp_nanos: 0,
    }),
    DataChannelMessage::MessageRevoke(MessageRevoke {
      message_id: mid(),
      timestamp_nanos: 0,
    }),
    DataChannelMessage::TypingIndicator(TypingIndicator { is_typing: true }),
    DataChannelMessage::MessageRead(MessageRead {
      message_ids: vec![mid()],
      timestamp_nanos: 0,
    }),
    DataChannelMessage::ForwardMessage(ForwardMessage {
      message_id: mid(),
      original_message_id: mid(),
      original_sender: uid(),
      content: "fwd".to_string(),
      timestamp_nanos: 0,
      room_id: None,
    }),
    DataChannelMessage::MessageReaction(MessageReaction {
      message_id: mid(),
      emoji: "👍".to_string(),
      action: ReactionAction::Add,
      timestamp_nanos: 0,
    }),
    DataChannelMessage::EcdhKeyExchange(EcdhKeyExchange {
      public_key: vec![0u8; 65],
      timestamp_nanos: 0,
    }),
    DataChannelMessage::AvatarRequest(AvatarRequest { user_id: uid() }),
    DataChannelMessage::AvatarData(AvatarData {
      user_id: uid(),
      data: vec![0u8; 10],
      mime_type: "image/png".to_string(),
      width: 64,
      height: 64,
    }),
    DataChannelMessage::Danmaku(Danmaku {
      content: "hi".to_string(),
      font_size: 24,
      color: 0xFFFFFF,
      position: message::types::DanmakuPosition::Scroll,
      video_time_ms: 0,
      timestamp_nanos: 0,
    }),
    DataChannelMessage::PlaybackProgress(PlaybackProgress {
      room_id: rid(),
      current_time_ms: 5000,
      duration_ms: 30000,
      is_paused: false,
      timestamp_nanos: 0,
    }),
    DataChannelMessage::SubtitleData(SubtitleData {
      room_id: rid(),
      entries: vec![SubtitleEntry {
        start_ms: 0,
        end_ms: 2000,
        text: "hello".to_string(),
      }],
    }),
    DataChannelMessage::SubtitleClear(SubtitleClear { room_id: rid() }),
    DataChannelMessage::DanmakuBatch(DanmakuBatch {
      room_id: rid(),
      entries: vec![],
    }),
    DataChannelMessage::TheaterChatText(TheaterChatText {
      room_id: rid(),
      sender_id: uid(),
      content: "tc".to_string(),
      timestamp_nanos: 0,
    }),
    DataChannelMessage::MediaStateUpdate(MediaStateUpdate {
      mic_enabled: true,
      camera_enabled: false,
      screen_sharing: false,
    }),
    DataChannelMessage::ReconnectingState(ReconnectingState { reconnecting: true }),
  ]
}
