//! Envelope serialization and chunking/reassembly tests

use message::{chat, envelope, transfer};

// ========================================================================
// Envelope basic serialization tests
// ========================================================================

#[test]
fn test_envelope_chat_message_serialize_roundtrip() {
  let chat_msg = chat::ChatMessage::new_text(
    "user-1".to_string(),
    vec!["user-2".to_string()],
    "test".to_string(),
  );
  let env = envelope::Envelope::new(
    "user-1".to_string(),
    vec!["user-2".to_string()],
    envelope::Payload::Chat(chat_msg),
  );
  let bytes = bitcode::serialize(&env).expect("serialization failed");
  let decoded: envelope::Envelope = bitcode::deserialize(&bytes).expect("deserialization failed");
  assert_eq!(decoded.from, "user-1");
  assert!(matches!(decoded.payload, envelope::Payload::Chat(_)));
}

#[test]
fn test_envelope_danmaku_serialize_roundtrip() {
  let danmaku = envelope::Danmaku {
    text: "danmaku test".to_string(),
    color: "#FF0000".to_string(),
    position: envelope::DanmakuPosition::Scroll,
    username: "alice".to_string(),
    video_time: 60.5,
  };
  let env = envelope::Envelope::new(
    "user-1".to_string(),
    vec![],
    envelope::Payload::Danmaku(danmaku),
  );
  let bytes = bitcode::serialize(&env).expect("serialization failed");
  let decoded: envelope::Envelope = bitcode::deserialize(&bytes).expect("deserialization failed");
  if let envelope::Payload::Danmaku(d) = &decoded.payload {
    assert_eq!(d.text, "danmaku test");
    assert_eq!(d.color, "#FF0000");
    assert!((d.video_time - 60.5).abs() < f64::EPSILON);
  } else {
    panic!("Payload type mismatch");
  }
}

#[test]
fn test_envelope_file_chunk_serialize_roundtrip() {
  let chunk = transfer::FileChunk {
    transfer_id: "transfer-1".to_string(),
    chunk_index: 42,
    data: vec![0xDE, 0xAD, 0xBE, 0xEF],
  };
  let env = envelope::Envelope::new(
    "user-1".to_string(),
    vec!["user-2".to_string()],
    envelope::Payload::FileChunk(chunk),
  );
  let bytes = bitcode::serialize(&env).expect("serialization failed");
  let decoded: envelope::Envelope = bitcode::deserialize(&bytes).expect("deserialization failed");
  if let envelope::Payload::FileChunk(c) = &decoded.payload {
    assert_eq!(c.transfer_id, "transfer-1");
    assert_eq!(c.chunk_index, 42);
    assert_eq!(c.data, vec![0xDE, 0xAD, 0xBE, 0xEF]);
  } else {
    panic!("Payload type mismatch");
  }
}

#[test]
fn test_envelope_key_exchange_serialize_roundtrip() {
  let key_data = envelope::KeyExchangeData {
    public_key: vec![1, 2, 3, 4, 5, 6, 7, 8],
  };
  let env = envelope::Envelope::new(
    "user-1".to_string(),
    vec!["user-2".to_string()],
    envelope::Payload::KeyExchange(key_data),
  );
  let bytes = bitcode::serialize(&env).expect("serialization failed");
  let decoded: envelope::Envelope = bitcode::deserialize(&bytes).expect("deserialization failed");
  if let envelope::Payload::KeyExchange(k) = &decoded.payload {
    assert_eq!(k.public_key, vec![1, 2, 3, 4, 5, 6, 7, 8]);
  } else {
    panic!("Payload type mismatch");
  }
}

#[test]
fn test_envelope_ack_serialize_roundtrip() {
  let env = envelope::Envelope::new(
    "user-1".to_string(),
    vec!["user-2".to_string()],
    envelope::Payload::Ack {
      message_id: "msg-123".to_string(),
    },
  );
  let bytes = bitcode::serialize(&env).expect("serialization failed");
  let decoded: envelope::Envelope = bitcode::deserialize(&bytes).expect("deserialization failed");
  if let envelope::Payload::Ack { message_id } = &decoded.payload {
    assert_eq!(message_id, "msg-123");
  } else {
    panic!("Payload type mismatch");
  }
}

// ========================================================================
// Envelope chunking/reassembly tests
// ========================================================================

#[test]
fn test_small_envelope_no_split() {
  let msg = chat::ChatMessage::new_text(
    "user-1".to_string(),
    vec!["user-2".to_string()],
    "short message".to_string(),
  );
  let env = envelope::Envelope::new(
    "user-1".to_string(),
    vec!["user-2".to_string()],
    envelope::Payload::Chat(msg),
  );
  let chunks = env
    .split(envelope::DEFAULT_CHUNK_THRESHOLD)
    .expect("chunking failed");
  // Small messages should not be chunked
  assert_eq!(chunks.len(), 1);
  // Direct deserialization should yield the original Envelope
  let decoded: envelope::Envelope =
    bitcode::deserialize(&chunks[0]).expect("deserialization failed");
  assert_eq!(decoded.from, "user-1");
  assert!(matches!(decoded.payload, envelope::Payload::Chat(_)));
}

#[test]
fn test_large_text_split_and_reassemble() {
  // Create an oversized text message (exceeding 16KB)
  let large_text = "A".repeat(50_000);
  let msg = chat::ChatMessage::new_text(
    "user-1".to_string(),
    vec!["user-2".to_string()],
    large_text.clone(),
  );
  let env = envelope::Envelope::new(
    "user-1".to_string(),
    vec!["user-2".to_string()],
    envelope::Payload::Chat(msg),
  );
  let original_id = env.id.clone();

  let chunks = env
    .split(envelope::DEFAULT_CHUNK_THRESHOLD)
    .expect("chunking failed");
  // Should be split into multiple chunks
  assert!(
    chunks.len() > 1,
    "Large message should be chunked, actual chunk count: {}",
    chunks.len()
  );

  // Reassemble using FragmentAssembler
  let mut assembler = envelope::FragmentAssembler::new();
  let mut result = None;
  for chunk_bytes in &chunks {
    let fragment_env: envelope::Envelope =
      bitcode::deserialize(chunk_bytes).expect("Fragment deserialization failed");
    if let envelope::Payload::Fragment(fragment) = fragment_env.payload {
      match assembler.push(fragment).expect("reassembly failed") {
        Some(complete) => {
          result = Some(complete);
          break;
        }
        None => continue,
      }
    } else {
      panic!("Payload after chunking should be Fragment");
    }
  }

  let reassembled = result.expect("reassembly should succeed");
  assert_eq!(reassembled.id, original_id);
  assert_eq!(reassembled.from, "user-1");
  if let envelope::Payload::Chat(chat_msg) = &reassembled.payload {
    if let chat::MessageContent::Text(text) = &chat_msg.content {
      assert_eq!(text.len(), 50_000);
      assert_eq!(text, &large_text);
    } else {
      panic!("Message content type mismatch");
    }
  } else {
    panic!("Payload type mismatch");
  }
}

#[test]
fn test_large_voice_split_and_reassemble() {
  // Create a large voice message (200KB of audio data)
  let audio_data = vec![0xABu8; 200 * 1024];
  let msg = chat::ChatMessage::new_voice(
    "user-1".to_string(),
    vec!["user-2".to_string()],
    audio_data.clone(),
    30_000,
  );
  let env = envelope::Envelope::new(
    "user-1".to_string(),
    vec!["user-2".to_string()],
    envelope::Payload::Chat(msg),
  );

  let chunks = env
    .split(envelope::DEFAULT_CHUNK_THRESHOLD)
    .expect("chunking failed");
  assert!(chunks.len() > 1, "Large voice message should be chunked");

  // Reassemble
  let mut assembler = envelope::FragmentAssembler::new();
  let mut result = None;
  for chunk_bytes in &chunks {
    let fragment_env: envelope::Envelope =
      bitcode::deserialize(chunk_bytes).expect("Fragment deserialization failed");
    if let envelope::Payload::Fragment(fragment) = fragment_env.payload {
      if let Some(complete) = assembler.push(fragment).expect("reassembly failed") {
        result = Some(complete);
      }
    }
  }

  let reassembled = result.expect("reassembly should succeed");
  if let envelope::Payload::Chat(chat_msg) = &reassembled.payload {
    if let chat::MessageContent::Voice { data, duration_ms } = &chat_msg.content {
      assert_eq!(data.len(), 200 * 1024);
      assert_eq!(*duration_ms, 30_000);
    } else {
      panic!("Message content type mismatch");
    }
  } else {
    panic!("Payload type mismatch");
  }
}

#[test]
fn test_large_image_split_and_reassemble() {
  // Create an image message with full-resolution data (500KB)
  let thumbnail = vec![0xFFu8; 4 * 1024]; // 4KB thumbnail
  let full_data = vec![0xAAu8; 500 * 1024]; // 500KB full image
  let meta = chat::ImageMeta {
    width: 1920,
    height: 1080,
    size: full_data.len() as u64,
    format: "jpeg".to_string(),
  };
  let mut msg = chat::ChatMessage::new_image(
    "user-1".to_string(),
    vec!["user-2".to_string()],
    thumbnail.clone(),
    meta.clone(),
  );
  // Manually set full_data
  if let chat::MessageContent::Image {
    full_data: ref mut fd,
    ..
  } = msg.content
  {
    *fd = Some(full_data.clone());
  }

  let env = envelope::Envelope::new(
    "user-1".to_string(),
    vec!["user-2".to_string()],
    envelope::Payload::Chat(msg),
  );

  let chunks = env
    .split(envelope::DEFAULT_CHUNK_THRESHOLD)
    .expect("chunking failed");
  assert!(chunks.len() > 1, "Large image message should be chunked");

  // Reassemble
  let mut assembler = envelope::FragmentAssembler::new();
  let mut result = None;
  for chunk_bytes in &chunks {
    let fragment_env: envelope::Envelope =
      bitcode::deserialize(chunk_bytes).expect("Fragment deserialization failed");
    if let envelope::Payload::Fragment(fragment) = fragment_env.payload {
      if let Some(complete) = assembler.push(fragment).expect("reassembly failed") {
        result = Some(complete);
      }
    }
  }

  let reassembled = result.expect("reassembly should succeed");
  if let envelope::Payload::Chat(chat_msg) = &reassembled.payload {
    if let chat::MessageContent::Image {
      thumbnail: t,
      meta: m,
      full_data: fd,
    } = &chat_msg.content
    {
      assert_eq!(t.len(), 4 * 1024);
      assert_eq!(m, &meta);
      assert_eq!(fd.as_ref().unwrap().len(), 500 * 1024);
    } else {
      panic!("Message content type mismatch");
    }
  } else {
    panic!("Payload type mismatch");
  }
}

#[test]
fn test_fragment_assembler_out_of_order() {
  // Test out-of-order chunk arrival
  let large_text = "B".repeat(100_000);
  let msg = chat::ChatMessage::new_text(
    "user-1".to_string(),
    vec!["user-2".to_string()],
    large_text.clone(),
  );
  let env = envelope::Envelope::new(
    "user-1".to_string(),
    vec!["user-2".to_string()],
    envelope::Payload::Chat(msg),
  );

  let chunks = env
    .split(envelope::DEFAULT_CHUNK_THRESHOLD)
    .expect("chunking failed");
  assert!(chunks.len() > 2, "Should have multiple chunks");

  // Reverse order to simulate out-of-order arrival
  let mut reversed_chunks = chunks.clone();
  reversed_chunks.reverse();

  let mut assembler = envelope::FragmentAssembler::new();
  let mut result = None;
  for chunk_bytes in &reversed_chunks {
    let fragment_env: envelope::Envelope =
      bitcode::deserialize(chunk_bytes).expect("Fragment deserialization failed");
    if let envelope::Payload::Fragment(fragment) = fragment_env.payload {
      if let Some(complete) = assembler.push(fragment).expect("reassembly failed") {
        result = Some(complete);
      }
    }
  }

  let reassembled = result.expect("out-of-order reassembly should succeed");
  if let envelope::Payload::Chat(chat_msg) = &reassembled.payload {
    if let chat::MessageContent::Text(text) = &chat_msg.content {
      assert_eq!(text, &large_text);
    } else {
      panic!("Message content type mismatch");
    }
  } else {
    panic!("Payload type mismatch");
  }
}

#[test]
fn test_fragment_assembler_pending_count() {
  let mut assembler = envelope::FragmentAssembler::new();
  assert_eq!(assembler.pending_count(), 0);

  // Add an incomplete group
  let fragment = envelope::EnvelopeFragment {
    group_id: "group-1".to_string(),
    chunk_index: 0,
    total_chunks: 3,
    data: vec![1, 2, 3],
  };
  let result = assembler.push(fragment).expect("push should succeed");
  assert!(result.is_none());
  assert_eq!(assembler.pending_count(), 1);

  // Cleanup
  assembler.remove_group("group-1");
  assert_eq!(assembler.pending_count(), 0);
}

#[test]
fn test_envelope_fragment_serialize_roundtrip() {
  let fragment = envelope::EnvelopeFragment {
    group_id: "group-1".to_string(),
    chunk_index: 2,
    total_chunks: 5,
    data: vec![0xDE, 0xAD, 0xBE, 0xEF],
  };
  let env = envelope::Envelope::new(
    "user-1".to_string(),
    vec!["user-2".to_string()],
    envelope::Payload::Fragment(fragment.clone()),
  );
  let bytes = bitcode::serialize(&env).expect("serialization failed");
  let decoded: envelope::Envelope = bitcode::deserialize(&bytes).expect("deserialization failed");
  if let envelope::Payload::Fragment(f) = &decoded.payload {
    assert_eq!(f.group_id, "group-1");
    assert_eq!(f.chunk_index, 2);
    assert_eq!(f.total_chunks, 5);
    assert_eq!(f.data, vec![0xDE, 0xAD, 0xBE, 0xEF]);
  } else {
    panic!("Payload type mismatch");
  }
}

// ========================================================================
// Error path tests
// ========================================================================

#[test]
fn test_fragment_assembler_index_out_of_range() {
  let mut assembler = envelope::FragmentAssembler::new();
  // chunk_index (5) exceeds total_chunks (3)
  let fragment = envelope::EnvelopeFragment {
    group_id: "group-err".to_string(),
    chunk_index: 5,
    total_chunks: 3,
    data: vec![1, 2, 3],
  };
  let result = assembler.push(fragment);
  assert!(result.is_err());
  assert!(result.unwrap_err().contains("out of range"));
}

#[test]
fn test_fragment_assembler_index_at_boundary() {
  let mut assembler = envelope::FragmentAssembler::new();
  // chunk_index == total_chunks (exactly at boundary, should fail)
  let fragment = envelope::EnvelopeFragment {
    group_id: "group-boundary".to_string(),
    chunk_index: 3,
    total_chunks: 3,
    data: vec![1, 2, 3],
  };
  let result = assembler.push(fragment);
  assert!(result.is_err());
}

#[test]
fn test_envelope_encrypted_payload_serialize_roundtrip() {
  let encrypted = envelope::EncryptedPayload {
    iv: vec![
      0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C,
    ],
    ciphertext: vec![0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE],
  };
  let env = envelope::Envelope::new(
    "user-1".to_string(),
    vec!["user-2".to_string()],
    envelope::Payload::Encrypted(encrypted.clone()),
  );
  let bytes = bitcode::serialize(&env).expect("serialization failed");
  let decoded: envelope::Envelope = bitcode::deserialize(&bytes).expect("deserialization failed");
  if let envelope::Payload::Encrypted(e) = &decoded.payload {
    assert_eq!(e.iv.len(), 12);
    assert_eq!(e.ciphertext, vec![0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE]);
  } else {
    panic!("Payload type mismatch");
  }
}

#[test]
fn test_envelope_typing_payload_serialize_roundtrip() {
  let typing = chat::TypingIndicator {
    user_id: "user-1".to_string(),
    is_typing: true,
  };
  let env = envelope::Envelope::new(
    "user-1".to_string(),
    vec!["user-2".to_string()],
    envelope::Payload::Typing(typing),
  );
  let bytes = bitcode::serialize(&env).expect("serialization failed");
  let decoded: envelope::Envelope = bitcode::deserialize(&bytes).expect("deserialization failed");
  if let envelope::Payload::Typing(t) = &decoded.payload {
    assert_eq!(t.user_id, "user-1");
    assert!(t.is_typing);
  } else {
    panic!("Payload type mismatch");
  }
}
