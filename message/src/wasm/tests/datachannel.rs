use super::*;

#[wasm_bindgen_test]
fn test_wasm_datachannel_chat_sticker_roundtrip() {
  use crate::datachannel::ChatSticker;
  use crate::types::MessageId;
  let msg = ChatSticker {
    message_id: MessageId::new(),
    pack_id: "pack_001".to_string(),
    sticker_id: "sticker_042".to_string(),
    reply_to: None,
    timestamp_nanos: 1_000_000_000,
  };
  roundtrip_datachannel(0x81, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_chat_voice_roundtrip() {
  use crate::datachannel::ChatVoice;
  use crate::types::MessageId;
  let msg = ChatVoice {
    message_id: MessageId::new(),
    audio_data: vec![0x00, 0x01, 0x02, 0x03],
    duration_ms: 3500,
    waveform: vec![10, 20, 30, 40],
    reply_to: None,
    timestamp_nanos: 1_000_000_000,
  };
  roundtrip_datachannel(0x82, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_chat_image_roundtrip() {
  use crate::datachannel::ChatImage;
  use crate::types::MessageId;
  let msg = ChatImage {
    message_id: MessageId::new(),
    image_data: vec![0xFF, 0xD8, 0xFF, 0xE0],
    thumbnail: vec![0x00, 0x01],
    width: 1920,
    height: 1080,
    reply_to: None,
    timestamp_nanos: 1_000_000_000,
  };
  roundtrip_datachannel(0x83, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_file_chunk_roundtrip() {
  use crate::datachannel::FileChunk;
  use crate::types::TransferId;
  let msg = FileChunk {
    transfer_id: TransferId::new(),
    chunk_index: 0,
    total_chunks: 5,
    data: vec![0xAB, 0xCD, 0xEF],
    chunk_hash: [0u8; 32],
  };
  roundtrip_datachannel(0x84, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_file_metadata_roundtrip() {
  use crate::datachannel::FileMetadata;
  use crate::types::{MessageId, TransferId};
  let msg = FileMetadata {
    message_id: MessageId::new(),
    transfer_id: TransferId::new(),
    filename: "document.pdf".to_string(),
    size: 1_048_576,
    mime_type: "application/pdf".to_string(),
    file_hash: [1u8; 32],
    total_chunks: 16,
    chunk_size: 65_536,
    reply_to: None,
    timestamp_nanos: 1_000_000_000,
  };
  roundtrip_datachannel(0x85, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_message_ack_roundtrip() {
  use crate::datachannel::{AckStatus, MessageAck};
  use crate::types::MessageId;
  let msg = MessageAck {
    message_id: MessageId::new(),
    status: AckStatus::Received,
    timestamp_nanos: 1_000_000_000,
  };
  roundtrip_datachannel(0x90, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_message_revoke_roundtrip() {
  use crate::datachannel::MessageRevoke;
  use crate::types::MessageId;
  let msg = MessageRevoke {
    message_id: MessageId::new(),
    timestamp_nanos: 1_000_000_000,
  };
  roundtrip_datachannel(0x91, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_typing_indicator_roundtrip() {
  use crate::datachannel::TypingIndicator;
  let msg = TypingIndicator { is_typing: true };
  roundtrip_datachannel(0x92, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_message_read_roundtrip() {
  use crate::datachannel::MessageRead;
  use crate::types::MessageId;
  let msg = MessageRead {
    message_ids: vec![MessageId::new(), MessageId::new()],
    timestamp_nanos: 1_000_000_000,
  };
  roundtrip_datachannel(0x93, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_forward_message_roundtrip() {
  use crate::datachannel::ForwardMessage;
  use crate::types::{MessageId, UserId};
  let msg = ForwardMessage {
    message_id: MessageId::new(),
    original_message_id: MessageId::new(),
    original_sender: UserId::new(),
    content: "Forwarded from WASM".to_string(),
    timestamp_nanos: 1_000_000_000,
  };
  roundtrip_datachannel(0x94, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_message_reaction_roundtrip() {
  use crate::datachannel::{MessageReaction, ReactionAction};
  use crate::types::MessageId;
  let msg = MessageReaction {
    message_id: MessageId::new(),
    emoji: "👍".to_string(),
    action: ReactionAction::Add,
    timestamp_nanos: 1_000_000_000,
  };
  roundtrip_datachannel(0x95, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_ecdh_key_exchange_roundtrip() {
  use crate::datachannel::EcdhKeyExchange;
  let msg = EcdhKeyExchange {
    public_key: vec![42u8; 65], // P-256 raw format: 65 bytes
    timestamp_nanos: 1_000_000_000,
  };
  roundtrip_datachannel(0xA0, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_avatar_request_roundtrip() {
  use crate::datachannel::AvatarRequest;
  use crate::types::UserId;
  let msg = AvatarRequest {
    user_id: UserId::new(),
  };
  roundtrip_datachannel(0xA1, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_avatar_data_roundtrip() {
  use crate::datachannel::AvatarData;
  use crate::types::UserId;
  let msg = AvatarData {
    user_id: UserId::new(),
    data: vec![0x89, 0x50, 0x4E, 0x47],
    mime_type: "image/png".to_string(),
    width: 128,
    height: 128,
  };
  roundtrip_datachannel(0xA2, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_danmaku_roundtrip() {
  use crate::datachannel::Danmaku;
  use crate::types::DanmakuPosition;
  let msg = Danmaku {
    content: "WASM danmaku!".to_string(),
    font_size: 24,
    color: 0xFFFFFF,
    position: DanmakuPosition::Scroll,
    video_time_ms: 12_000,
    timestamp_nanos: 1_000_000_000,
  };
  roundtrip_datachannel(0xB0, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_playback_progress_roundtrip() {
  use crate::datachannel::PlaybackProgress;
  use crate::types::RoomId;
  let msg = PlaybackProgress {
    room_id: RoomId::new(),
    current_time_ms: 45_000,
    duration_ms: 3_600_000,
    is_paused: false,
    timestamp_nanos: 1_000_000_000,
  };
  roundtrip_datachannel(0xB1, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_subtitle_data_roundtrip() {
  use crate::datachannel::{SubtitleData, SubtitleEntry};
  use crate::types::RoomId;
  let msg = SubtitleData {
    room_id: RoomId::new(),
    entries: vec![SubtitleEntry {
      start_ms: 1000,
      end_ms: 3000,
      text: "WASM subtitle".to_string(),
    }],
  };
  roundtrip_datachannel(0xB2, &msg);
}

#[wasm_bindgen_test]
fn test_wasm_datachannel_subtitle_clear_roundtrip() {
  use crate::datachannel::SubtitleClear;
  use crate::types::RoomId;
  let msg = SubtitleClear {
    room_id: RoomId::new(),
  };
  roundtrip_datachannel(0xB3, &msg);
}
