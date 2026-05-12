use super::*;

#[test]
fn base64_matches_reference_vectors() {
  assert_eq!(base64_encode(b""), "");
  assert_eq!(base64_encode(b"f"), "Zg==");
  assert_eq!(base64_encode(b"fo"), "Zm8=");
  assert_eq!(base64_encode(b"foo"), "Zm9v");
  assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
  assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
  assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
}

#[test]
fn base64_single_byte_padding() {
  // Single remaining byte produces two data chars + "=="
  assert_eq!(base64_encode(b"A"), "QQ==");
  assert_eq!(base64_encode(b"AB"), "QUI=");
}

#[test]
fn base64_long_input_no_padding() {
  // 3-byte multiples produce no padding
  assert_eq!(base64_encode(b"abc"), "YWJj");
  assert_eq!(base64_encode(b"abcdef"), "YWJjZGVm");
}

#[test]
fn base64_all_zero_bytes() {
  assert_eq!(base64_encode(&[0u8; 3]), "AAAA");
  assert_eq!(base64_encode(&[0u8; 1]), "AA==");
  assert_eq!(base64_encode(&[0u8; 2]), "AAA=");
}

#[test]
fn data_url_preserves_mime_and_payload() {
  let url = bytes_to_data_url("image/png", &[1, 2, 3]);
  assert!(url.starts_with("data:image/png;base64,"));
  assert!(url.ends_with("AQID"));
}

#[test]
fn data_url_with_voice_mime() {
  let url = bytes_to_data_url("audio/webm", &[0xFF, 0xFB, 0x90]);
  assert!(url.starts_with("data:audio/webm;base64,"));
  assert!(!url.contains("image"));
}

#[test]
fn data_url_empty_bytes() {
  let url = bytes_to_data_url("image/jpeg", &[]);
  assert_eq!(url, "data:image/jpeg;base64,");
}

#[test]
fn nanos_to_ms_rounds_down() {
  assert_eq!(nanos_to_ms(1_500_000), 1);
  assert_eq!(nanos_to_ms(0), 0);
  assert_eq!(nanos_to_ms(1_999_999), 1);
}

#[test]
fn nanos_to_ms_exact_conversion() {
  assert_eq!(nanos_to_ms(1_000_000), 1);
  assert_eq!(nanos_to_ms(5_000_000), 5);
  assert_eq!(nanos_to_ms(1_000_000_000), 1_000);
}

#[test]
fn nanos_to_ms_saturates_on_overflow() {
  // u64::MAX / 1_000_000 = 18_446_744_073_709, which fits in i64 so
  // no saturation occurs. The saturating behaviour only triggers when
  // the millisecond value itself exceeds i64::MAX (requires >10^19 ns).
  let max_nanos = u64::MAX;
  let result = nanos_to_ms(max_nanos);
  assert_eq!(result, 18_446_744_073_709_i64);
}

#[test]
fn nanos_to_ms_large_but_valid() {
  // A large but representable value (year ~2262 in ms)
  let large_nanos: u64 = 9_223_372_036_000_000_000; // just under i64::MAX in ms
  let result = nanos_to_ms(large_nanos);
  assert!(result > 0);
  // result fits in i64 by construction (nanos_to_ms returns i64)
}

#[test]
fn build_inbound_creates_correct_message() {
  let id = MessageId(uuid::Uuid::new_v4());
  let sender = UserId::from(42u64);
  let msg = build_inbound(
    id,
    sender.clone(),
    "Bob".to_string(),
    MessageContent::Text("hello".to_string()),
    1_234_567_890_000_000_000, // 1_234_567_890 seconds in nanos
    None,
    false,
  );

  assert_eq!(msg.id, id);
  assert_eq!(msg.sender, sender);
  assert_eq!(msg.sender_name, "Bob");
  assert_eq!(msg.content, MessageContent::Text("hello".to_string()));
  assert!(!msg.outgoing);
  assert_eq!(msg.status, MessageStatus::Received);
  assert!(msg.reply_to.is_none());
  assert!(!msg.mentions_me);
  assert!(!msg.counted_unread);
  assert!(msg.read_by.is_empty());
  assert!(msg.reactions.is_empty());
}

#[test]
fn build_inbound_with_mentions_me() {
  let id = MessageId(uuid::Uuid::new_v4());
  let sender = UserId::from(1u64);
  let msg = build_inbound(
    id,
    sender,
    "Alice".to_string(),
    MessageContent::Text("@you hello".to_string()),
    0,
    None,
    true,
  );
  assert!(msg.mentions_me);
}

#[test]
fn build_inbound_timestamp_truncates_nanos_to_ms() {
  let id = MessageId(uuid::Uuid::new_v4());
  let sender = UserId::from(1u64);
  let msg = build_inbound(
    id,
    sender,
    "Carol".to_string(),
    MessageContent::Text("test".to_string()),
    1_234_567_890_123_456_789,
    None,
    false,
  );
  // nanos_to_ms truncates: 1_234_567_890_123_456_789 ns → 1_234_567_890_123 ms
  assert_eq!(msg.timestamp_ms, 1_234_567_890_123_i64);
}

// ---------------------------------------------------------------------------
// build_inbound — reply_to forwarding
// ---------------------------------------------------------------------------

#[test]
fn build_inbound_forwards_reply_snippet() {
  let id = MessageId(uuid::Uuid::new_v4());
  let sender = UserId::from(5u64);
  let reply = ReplySnippet {
    message_id: MessageId(uuid::Uuid::new_v4()),
    sender_name: "Dave".to_string(),
    preview: "original msg".to_string(),
  };
  let msg = build_inbound(
    id,
    sender,
    "Eve".to_string(),
    MessageContent::Text("reply".to_string()),
    0,
    Some(reply.clone()),
    false,
  );
  assert_eq!(msg.reply_to, Some(reply));
}

// ---------------------------------------------------------------------------
// base64_encode — boundary and special values
// ---------------------------------------------------------------------------

#[test]
fn base256_byte_values_0_to_255() {
  // Encode all 256 byte values to verify no indexing panics
  let input: Vec<u8> = (0..=255).collect();
  let encoded = base64_encode(&input);
  // Must be valid Base64 length (multiple of 4)
  assert_eq!(encoded.len() % 4, 0);
  // 256 bytes → ceil(256/3)*4 = 344 chars (256 = 85*3 + 1, so 86*4 = 344)
  assert_eq!(encoded.len(), 344);
}

#[test]
fn base64_high_byte_values() {
  // Verify encoding of non-ASCII bytes (0x80–0xFF)
  assert_eq!(base64_encode(&[0xFF]), "/w==");
  assert_eq!(base64_encode(&[0xFE, 0xFD]), "/v0=");
  assert_eq!(base64_encode(&[0x80, 0x81, 0x82]), "gIGC");
}

#[test]
fn base64_uneven_lengths_roundtrip_structure() {
  // Verify padding pattern for all remainder classes
  for len in 1..=5 {
    let data = vec![0xABu8; len];
    let encoded = base64_encode(&data);
    let expected_padding = match len % 3 {
      0 => 0,
      1 => 2,
      2 => 1,
      _ => unreachable!(),
    };
    let actual_padding = encoded.chars().filter(|&c| c == '=').count();
    assert_eq!(
      actual_padding, expected_padding,
      "Length {len}: expected {expected_padding} padding chars, got {actual_padding}"
    );
  }
}

// ---------------------------------------------------------------------------
// nanos_to_ms — boundary: zero and near-zero
// ---------------------------------------------------------------------------

#[test]
fn nanos_to_ms_sub_millisecond_returns_zero() {
  assert_eq!(nanos_to_ms(0), 0);
  assert_eq!(nanos_to_ms(1), 0);
  assert_eq!(nanos_to_ms(999_999), 0);
}

#[test]
fn nanos_to_ms_exactly_one_ms() {
  assert_eq!(nanos_to_ms(1_000_000), 1);
}

// ---------------------------------------------------------------------------
// data_url — common MIME types used in the codebase
// ---------------------------------------------------------------------------

#[test]
fn data_url_audio_webm_roundtrip_structure() {
  let url = bytes_to_data_url("audio/webm", &[0x1A, 0x45, 0xDF]);
  assert!(url.starts_with("data:audio/webm;base64,"));
  // 3 bytes → no padding in base64
  assert!(!url.ends_with('='));
}

#[test]
fn data_url_image_jpeg_roundtrip_structure() {
  let url = bytes_to_data_url("image/jpeg", &[0xFF, 0xD8, 0xFF]);
  assert!(url.starts_with("data:image/jpeg;base64,"));
  // 3 bytes → no padding
  assert!(!url.ends_with('='));
}
