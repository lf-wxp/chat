//! Chat message serialization tests

use message::chat;

#[test]
fn test_text_message_serialize_roundtrip() {
  let msg = chat::ChatMessage::new_text(
    "user-1".to_string(),
    vec!["user-2".to_string()],
    "Hello, World!".to_string(),
  );
  let bytes = bitcode::serialize(&msg).expect("Serialization failed");
  let decoded: chat::ChatMessage = bitcode::deserialize(&bytes).expect("Deserialization failed");
  assert_eq!(decoded.from, "user-1");
  assert_eq!(decoded.to, vec!["user-2"]);
  if let chat::MessageContent::Text(text) = &decoded.content {
    assert_eq!(text, "Hello, World!");
  } else {
    panic!("Message type mismatch");
  }
}

#[test]
fn test_sticker_message_serialize_roundtrip() {
  let msg = chat::ChatMessage::new_sticker(
    "user-1".to_string(),
    vec!["user-2".to_string()],
    "pack-1".to_string(),
    "sticker-1".to_string(),
  );
  let bytes = bitcode::serialize(&msg).expect("Serialization failed");
  let decoded: chat::ChatMessage = bitcode::deserialize(&bytes).expect("Deserialization failed");
  if let chat::MessageContent::Sticker {
    pack_id,
    sticker_id,
  } = &decoded.content
  {
    assert_eq!(pack_id, "pack-1");
    assert_eq!(sticker_id, "sticker-1");
  } else {
    panic!("Message type mismatch");
  }
}

#[test]
fn test_voice_message_serialize_roundtrip() {
  let audio_data = vec![1u8, 2, 3, 4, 5];
  let msg = chat::ChatMessage::new_voice(
    "user-1".to_string(),
    vec!["user-2".to_string()],
    audio_data.clone(),
    3000,
  );
  let bytes = bitcode::serialize(&msg).expect("Serialization failed");
  let decoded: chat::ChatMessage = bitcode::deserialize(&bytes).expect("Deserialization failed");
  if let chat::MessageContent::Voice { data, duration_ms } = &decoded.content {
    assert_eq!(data, &audio_data);
    assert_eq!(*duration_ms, 3000);
  } else {
    panic!("Message type mismatch");
  }
}

#[test]
fn test_image_message_serialize_roundtrip() {
  let thumbnail = vec![0xFFu8, 0xD8, 0xFF, 0xE0];
  let meta = chat::ImageMeta {
    width: 1920,
    height: 1080,
    size: 2_000_000,
    format: "jpeg".to_string(),
  };
  let msg = chat::ChatMessage::new_image(
    "user-1".to_string(),
    vec!["user-2".to_string()],
    thumbnail.clone(),
    meta.clone(),
  );
  let bytes = bitcode::serialize(&msg).expect("Serialization failed");
  let decoded: chat::ChatMessage = bitcode::deserialize(&bytes).expect("Deserialization failed");
  if let chat::MessageContent::Image {
    thumbnail: t,
    meta: m,
    full_data,
  } = &decoded.content
  {
    assert_eq!(t, &thumbnail);
    assert_eq!(m, &meta);
    assert!(full_data.is_none());
  } else {
    panic!("Message type mismatch");
  }
}

#[test]
fn test_file_meta_serialize_roundtrip() {
  let file_meta = chat::FileMeta {
    name: "document.pdf".to_string(),
    size: 1_048_576,
    mime_type: "application/pdf".to_string(),
    transfer_id: Some("transfer-1".to_string()),
  };
  let content = chat::MessageContent::File(file_meta.clone());
  let bytes = bitcode::serialize(&content).expect("Serialization failed");
  let decoded: chat::MessageContent = bitcode::deserialize(&bytes).expect("Deserialization failed");
  if let chat::MessageContent::File(f) = decoded {
    assert_eq!(f.name, "document.pdf");
    assert_eq!(f.size, 1_048_576);
    assert_eq!(f.transfer_id, Some("transfer-1".to_string()));
  } else {
    panic!("Message type mismatch");
  }
}

#[test]
fn test_typing_indicator_serialize_roundtrip() {
  let indicator = chat::TypingIndicator {
    user_id: "user-1".to_string(),
    is_typing: true,
  };
  let bytes = bitcode::serialize(&indicator).expect("Serialization failed");
  let decoded: chat::TypingIndicator =
    bitcode::deserialize(&bytes).expect("Deserialization failed");
  assert_eq!(decoded.user_id, "user-1");
  assert!(decoded.is_typing);
}
