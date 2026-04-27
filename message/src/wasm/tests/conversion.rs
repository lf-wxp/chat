use super::*;

// ========================================================================
// ArrayBuffer Conversion Error Path Tests
// ========================================================================

#[wasm_bindgen_test]
fn test_array_buffer_conversion() {
  let data = vec![1, 2, 3, 4, 5];

  // Vec -> Uint8Array
  let uint8 = vec_to_uint8_array(&data);
  assert_eq!(uint8.to_vec(), data);

  // Vec -> ArrayBuffer
  let buffer = vec_to_array_buffer(&data);
  let uint8_from_buffer = Uint8Array::new(&buffer);
  assert_eq!(uint8_from_buffer.to_vec(), data);

  // Uint8Array -> Vec
  let vec_from_uint8 = uint8_array_to_vec(&uint8);
  assert_eq!(vec_from_uint8, data);

  // ArrayBuffer -> Vec
  let vec_from_buffer = array_buffer_to_vec(&buffer);
  assert_eq!(vec_from_buffer, data);
}

#[wasm_bindgen_test]
fn test_array_buffer_empty_conversion() {
  // Empty Vec -> ArrayBuffer -> Vec roundtrip
  let data: Vec<u8> = vec![];
  let buffer = vec_to_array_buffer(&data);
  let result = array_buffer_to_vec(&buffer);
  assert_eq!(result, data);
  assert!(result.is_empty());
}

#[wasm_bindgen_test]
fn test_uint8_array_empty_conversion() {
  // Empty Vec -> Uint8Array -> Vec roundtrip
  let data: Vec<u8> = vec![];
  let uint8 = vec_to_uint8_array(&data);
  let result = uint8_array_to_vec(&uint8);
  assert_eq!(result, data);
  assert!(result.is_empty());
}

#[wasm_bindgen_test]
fn test_array_buffer_large_data_conversion() {
  // Large data (1MB) roundtrip through ArrayBuffer
  let data: Vec<u8> = (0..=255).cycle().take(1_000_000).collect();
  let buffer = vec_to_array_buffer(&data);
  let result = array_buffer_to_vec(&buffer);
  assert_eq!(result.len(), data.len());
  assert_eq!(result, data);
}

#[wasm_bindgen_test]
fn test_encode_from_buffer_empty_payload() {
  // Empty ArrayBuffer should fail encoding (same as empty slice)
  let empty_buffer = vec_to_array_buffer(&[]);
  let result = encode_message_from_buffer(0x01, &empty_buffer);
  assert!(result.is_err());
  let err_msg = result.unwrap_err().as_string().unwrap();
  assert!(
    err_msg.contains("empty"),
    "Expected 'empty' in error: {err_msg}"
  );
}

#[wasm_bindgen_test]
fn test_decode_from_buffer_invalid_magic() {
  // ArrayBuffer with invalid magic number
  let bad_frame = vec_to_array_buffer(&[0xAA, 0xBB, 0x01, 0xFF]);
  let result = decode_message_from_buffer(&bad_frame);
  assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_decode_from_buffer_too_short() {
  // ArrayBuffer too short for a valid frame
  let short_buffer = vec_to_array_buffer(&[0xBC]);
  let result = decode_message_from_buffer(&short_buffer);
  assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_decode_from_buffer_empty() {
  // Empty ArrayBuffer
  let empty_buffer = vec_to_array_buffer(&[]);
  let result = decode_message_from_buffer(&empty_buffer);
  assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_encode_decode_from_buffer_roundtrip() {
  // Valid encode → decode roundtrip through ArrayBuffer API
  let payload = vec![10, 20, 30, 40, 50];
  let payload_buffer = vec_to_array_buffer(&payload);
  let encoded = encode_message_from_buffer(0x05, &payload_buffer).expect("Failed to encode");

  let encoded_buffer = encoded.buffer();
  let decoded = decode_message_from_buffer(&encoded_buffer).expect("Failed to decode");

  let obj = Object::from(decoded);
  let msg_type = js_sys::Reflect::get(&obj, &wasm_bindgen::JsValue::from_str("messageType"))
    .expect("Failed to get messageType")
    .as_f64()
    .expect("messageType is not a number") as u8;
  assert_eq!(msg_type, 0x05);

  let payload_val = js_sys::Reflect::get(&obj, &wasm_bindgen::JsValue::from_str("payload"))
    .expect("Failed to get payload");
  let decoded_payload = Uint8Array::from(payload_val).to_vec();
  assert_eq!(decoded_payload, payload);
}

// ========================================================================
// Chunking Edge Case Tests
// ========================================================================

#[wasm_bindgen_test]
fn test_needs_chunking_boundary() {
  // Exactly at threshold — should NOT need chunking
  assert!(!needs_chunking(64 * 1024));
  // One byte over — should need chunking
  assert!(needs_chunking(64 * 1024 + 1));
  // Zero — should NOT need chunking
  assert!(!needs_chunking(0));
  // One byte — should NOT need chunking
  assert!(!needs_chunking(1));
}

#[wasm_bindgen_test]
fn test_calculate_chunk_count_edge_cases() {
  // Exact multiples of chunk size
  assert_eq!(calculate_chunk_count(64 * 1024), 1);
  assert_eq!(calculate_chunk_count(128 * 1024), 2);
  assert_eq!(calculate_chunk_count(192 * 1024), 3);

  // Off-by-one
  assert_eq!(calculate_chunk_count(64 * 1024 - 1), 1);
  assert_eq!(calculate_chunk_count(64 * 1024 + 1), 2);
  assert_eq!(calculate_chunk_count(128 * 1024 - 1), 2);
  assert_eq!(calculate_chunk_count(128 * 1024 + 1), 3);

  // Very large payload
  assert_eq!(calculate_chunk_count(10 * 1024 * 1024), 160); // 10MB
}

// ========================================================================
// Additional WASM Boundary Tests (CR-P1-001)
// ========================================================================

#[wasm_bindgen_test]
fn test_encode_decode_stress_many_iterations() {
  // Stress test: encode/decode many times to ensure stability
  let original_payload: Vec<u8> = (0..=255).cycle().take(1000).collect();

  let mut current = original_payload.clone();
  for i in 0..50 {
    let encoded = encode_message((i % 256) as u8, &current).expect("encode should succeed");
    let decoded = decode_message(&encoded.to_vec()).expect("decode should succeed");

    let obj = Object::from(decoded);
    let payload_val = js_sys::Reflect::get(&obj, &wasm_bindgen::JsValue::from_str("payload"))
      .expect("Failed to get payload");
    current = Uint8Array::from(payload_val).to_vec();
  }

  // After 50 roundtrips, payload should still match original
  assert_eq!(current, original_payload);
}

#[wasm_bindgen_test]
fn test_payload_with_null_bytes() {
  // Payload containing null bytes (0x00) throughout
  let payload = vec![0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00];
  let encoded = encode_message(0x42, &payload).expect("encode should succeed");
  let decoded = decode_message(&encoded.to_vec()).expect("decode should succeed");

  let obj = Object::from(decoded);
  let payload_val = js_sys::Reflect::get(&obj, &wasm_bindgen::JsValue::from_str("payload"))
    .expect("Failed to get payload");
  let decoded_payload = Uint8Array::from(payload_val).to_vec();
  assert_eq!(decoded_payload, payload);
}

#[wasm_bindgen_test]
fn test_payload_all_zeros() {
  // Payload of all zeros
  let payload = vec![0u8; 1000];
  let encoded = encode_message(0x42, &payload).expect("encode should succeed");
  let decoded = decode_message(&encoded.to_vec()).expect("decode should succeed");

  let obj = Object::from(decoded);
  let payload_val = js_sys::Reflect::get(&obj, &wasm_bindgen::JsValue::from_str("payload"))
    .expect("Failed to get payload");
  let decoded_payload = Uint8Array::from(payload_val).to_vec();
  assert_eq!(decoded_payload, payload);
}

#[wasm_bindgen_test]
fn test_payload_all_ones() {
  // Payload of all 0xFF bytes
  let payload = vec![0xFFu8; 1000];
  let encoded = encode_message(0x42, &payload).expect("encode should succeed");
  let decoded = decode_message(&encoded.to_vec()).expect("decode should succeed");

  let obj = Object::from(decoded);
  let payload_val = js_sys::Reflect::get(&obj, &wasm_bindgen::JsValue::from_str("payload"))
    .expect("Failed to get payload");
  let decoded_payload = Uint8Array::from(payload_val).to_vec();
  assert_eq!(decoded_payload, payload);
}

#[wasm_bindgen_test]
fn test_message_type_boundary_values() {
  // Test all boundary values for message_type
  let boundary_types: Vec<u8> = vec![
    0x00, 0x01, // Min and min+1
    0x7E, 0x7F, // Signaling/DataChannel boundary
    0x80, 0x81, // DataChannel start
    0xFE, 0xFF, // Max-1 and Max
  ];

  for msg_type in boundary_types {
    let payload = vec![0xAB, 0xCD, 0xEF];
    let encoded = encode_message(msg_type, &payload)
      .expect(&format!("encode should succeed for type 0x{msg_type:02X}"));
    let decoded = decode_message(&encoded.to_vec())
      .expect(&format!("decode should succeed for type 0x{msg_type:02X}"));

    let obj = Object::from(decoded);
    let decoded_type = js_sys::Reflect::get(&obj, &wasm_bindgen::JsValue::from_str("messageType"))
      .expect("Failed to get messageType")
      .as_f64()
      .expect("messageType is not a number") as u8;
    assert_eq!(decoded_type, msg_type);
  }
}

#[wasm_bindgen_test]
fn test_encode_message_consistent_output() {
  // Same input should always produce same output (deterministic encoding)
  let payload = vec![1, 2, 3, 4, 5];
  let msg_type: u8 = 0x42;

  let encoded1 = encode_message(msg_type, &payload).expect("encode 1");
  let encoded2 = encode_message(msg_type, &payload).expect("encode 2");

  assert_eq!(encoded1.to_vec(), encoded2.to_vec());
}

#[wasm_bindgen_test]
fn test_signaling_discriminator_range_wasm() {
  // Verify all SignalingMessage types use discriminators < 0x80 in WASM context
  use crate::signaling::SignalingMessage;

  let messages: Vec<SignalingMessage> = vec![
    SignalingMessage::TokenAuth(crate::signaling::TokenAuth {
      token: String::new(),
    }),
    SignalingMessage::Ping(crate::signaling::Ping {}),
    SignalingMessage::Pong(crate::signaling::Pong {}),
    SignalingMessage::UserLogout(crate::signaling::UserLogout {}),
    SignalingMessage::SessionInvalidated(crate::signaling::SessionInvalidated {}),
  ];

  for msg in &messages {
    let disc = msg.discriminator();
    assert!(
      disc < 0x80,
      "SignalingMessage discriminator 0x{:02X} should be < 0x80",
      disc
    );
  }
}

#[wasm_bindgen_test]
fn test_datachannel_discriminator_range_wasm() {
  // Verify all DataChannelMessage types use discriminators >= 0x80 in WASM context
  use crate::datachannel::DataChannelMessage;
  use crate::types::MessageId;

  let messages: Vec<DataChannelMessage> = vec![
    DataChannelMessage::ChatText(crate::datachannel::ChatText {
      message_id: MessageId::new(),
      content: String::new(),
      reply_to: None,
      timestamp_nanos: 0,
    }),
    DataChannelMessage::TypingIndicator(crate::datachannel::TypingIndicator { is_typing: true }),
    DataChannelMessage::MessageRead(crate::datachannel::MessageRead {
      message_ids: vec![],
      timestamp_nanos: 0,
    }),
  ];

  for msg in &messages {
    let disc = msg.discriminator();
    assert!(
      disc >= 0x80,
      "DataChannelMessage discriminator 0x{:02X} should be >= 0x80",
      disc
    );
  }
}

#[wasm_bindgen_test]
fn test_frame_structure_verification() {
  // Verify the exact structure of the encoded frame
  let payload = vec![0xDE, 0xAD, 0xBE, 0xEF];
  let msg_type: u8 = 0x42;
  let encoded = encode_message(msg_type, &payload).expect("encode should succeed");
  let bytes = encoded.to_vec();

  // Frame structure: [MAGIC_HI, MAGIC_LO, MSG_TYPE, ...PAYLOAD]
  assert_eq!(bytes.len(), 3 + payload.len()); // 2 magic + 1 type + payload
  assert_eq!(bytes[0], 0xBC); // Magic high byte
  assert_eq!(bytes[1], 0xBC); // Magic low byte
  assert_eq!(bytes[2], msg_type); // Message type
  assert_eq!(&bytes[3..], &payload[..]); // Payload
}

#[wasm_bindgen_test]
fn test_consecutive_different_messages() {
  // Encode and decode multiple different messages in sequence
  let messages: Vec<(u8, Vec<u8>)> = vec![
    (0x00, vec![1, 2, 3]),
    (0x42, vec![4, 5, 6]),
    (0x80, vec![7, 8, 9]),
    (0xFF, vec![10, 11, 12]),
  ];

  for (msg_type, payload) in messages {
    let encoded = encode_message(msg_type, &payload).expect("encode should succeed");
    let decoded = decode_message(&encoded.to_vec()).expect("decode should succeed");

    let obj = Object::from(decoded);
    let decoded_type = js_sys::Reflect::get(&obj, &wasm_bindgen::JsValue::from_str("messageType"))
      .expect("Failed to get messageType")
      .as_f64()
      .expect("messageType is not a number") as u8;

    let payload_val = js_sys::Reflect::get(&obj, &wasm_bindgen::JsValue::from_str("payload"))
      .expect("Failed to get payload");
    let decoded_payload = Uint8Array::from(payload_val).to_vec();

    assert_eq!(decoded_type, msg_type);
    assert_eq!(decoded_payload, payload);
  }
}

#[wasm_bindgen_test]
fn test_very_small_payload_sizes() {
  // Test payloads of various small sizes
  for size in 1..=16 {
    let payload: Vec<u8> = (0..size as u8).collect();
    let encoded =
      encode_message(0x01, &payload).expect(&format!("encode should succeed for size {size}"));
    let decoded =
      decode_message(&encoded.to_vec()).expect(&format!("decode should succeed for size {size}"));

    let obj = Object::from(decoded);
    let payload_val = js_sys::Reflect::get(&obj, &wasm_bindgen::JsValue::from_str("payload"))
      .expect("Failed to get payload");
    let decoded_payload = Uint8Array::from(payload_val).to_vec();
    assert_eq!(decoded_payload.len(), size);
  }
}
