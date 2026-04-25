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
