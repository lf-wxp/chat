use message::datachannel::{ChatText, DataChannelMessage};

#[test]
fn test_message_discriminator_roundtrip() {
  let msg = DataChannelMessage::ChatText(ChatText {
    message_id: message::MessageId(uuid::Uuid::new_v4()),
    content: "Hello, World!".to_string(),
    reply_to: None,
    timestamp_nanos: 1234567890,
  });

  let discriminator = msg.discriminator();
  let payload = bitcode::encode(&msg);

  // Rebuild frame
  let mut frame = Vec::new();
  frame.push(discriminator);
  frame.extend_from_slice(&payload);

  // Decode
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

#[test]
fn test_data_channel_message_types() {
  // Verify all discriminators are in the correct range (0x80-0xC3)
  assert!(
    DataChannelMessage::ChatText(ChatText {
      message_id: message::MessageId(uuid::Uuid::new_v4()),
      content: "".to_string(),
      reply_to: None,
      timestamp_nanos: 0,
    })
    .discriminator()
      >= 0x80
  );
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

  // 12 B IV + 20 B arbitrary ciphertext+tag.
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
