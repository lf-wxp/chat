use super::*;

#[wasm_bindgen_test]
fn test_encode_decode_frame() {
  let payload = vec![1, 2, 3, 4, 5];
  let result = encode_message(0x00, &payload).expect("Failed to encode");
  let bytes = result.to_vec();

  // Check magic number
  assert_eq!(bytes[0], 0xBC);
  assert_eq!(bytes[1], 0xBC);
  // Check message type
  assert_eq!(bytes[2], 0x00);
  // Check payload
  assert_eq!(&bytes[3..], &payload[..]);
}

#[wasm_bindgen_test]
fn test_decode_message_success() {
  // Build a valid frame manually
  let mut frame_bytes = vec![0xBC, 0xBC, 0x01]; // magic + type
  frame_bytes.extend_from_slice(&[10, 20, 30, 40]);

  let result = decode_message(&frame_bytes).expect("Failed to decode");
  let obj = Object::from(result);

  let msg_type = js_sys::Reflect::get(&obj, &wasm_bindgen::JsValue::from_str("messageType"))
    .expect("Failed to get messageType")
    .as_f64()
    .expect("messageType is not a number") as u8;
  assert_eq!(msg_type, 0x01);

  let payload_val = js_sys::Reflect::get(&obj, &wasm_bindgen::JsValue::from_str("payload"))
    .expect("Failed to get payload");
  let payload = Uint8Array::from(payload_val);
  assert_eq!(payload.to_vec(), vec![10, 20, 30, 40]);
}

#[wasm_bindgen_test]
fn test_decode_invalid_magic() {
  let frame_bytes = vec![0xAA, 0xBB, 0x01, 10, 20];
  let result = decode_message(&frame_bytes);
  assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_decode_too_short() {
  let frame_bytes = vec![0xBC, 0xBC]; // Only magic, no type
  let result = decode_message(&frame_bytes);
  assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_encode_empty_payload() {
  let payload: Vec<u8> = vec![];
  let result = encode_message(0x00, &payload);

  // Empty payloads should return an error
  assert!(result.is_err());
  let error = result.unwrap_err();
  assert!(error.as_string().unwrap().contains("empty"));
}

#[wasm_bindgen_test]
fn test_magic_number_constants() {
  assert_eq!(get_magic_number(), 0xBCBC);
  assert_eq!(get_magic_number_bytes(), vec![0xBC, 0xBC]);
  assert_eq!(get_header_size(), 3);
}

#[wasm_bindgen_test]
fn test_chunking_functions() {
  assert_eq!(get_max_chunk_size(), 64 * 1024);
  assert_eq!(get_chunking_threshold(), 64 * 1024);
  assert_eq!(get_header_size(), 3);

  assert!(!needs_chunking(1000));
  assert!(!needs_chunking(64 * 1024));
  assert!(needs_chunking(64 * 1024 + 1));
  assert!(needs_chunking(100_000));

  assert_eq!(calculate_chunk_count(0), 1);
  assert_eq!(calculate_chunk_count(1000), 1);
  assert_eq!(calculate_chunk_count(64 * 1024), 1);
  assert_eq!(calculate_chunk_count(64 * 1024 + 1), 2);
  assert_eq!(calculate_chunk_count(128 * 1024), 2);
  assert_eq!(calculate_chunk_count(128 * 1024 + 1), 3);
}

#[wasm_bindgen_test]
fn test_roundtrip_with_different_types() {
  for msg_type in 0x00..=0x10 {
    let payload = vec![msg_type, msg_type + 1, msg_type + 2];
    let encoded = encode_message(msg_type, &payload).expect("Failed to encode");
    let decoded = decode_message(&encoded.to_vec()).expect("Failed to decode");

    let obj = Object::from(decoded);
    let decoded_type = js_sys::Reflect::get(&obj, &wasm_bindgen::JsValue::from_str("messageType"))
      .expect("Failed to get messageType")
      .as_f64()
      .expect("messageType is not a number") as u8;
    assert_eq!(decoded_type, msg_type);
  }
}

#[wasm_bindgen_test]
fn test_large_payload() {
  let payload: Vec<u8> = (0..=255).cycle().take(100_000).collect();
  let encoded = encode_message(0x42, &payload).expect("Failed to encode");
  let decoded = decode_message(&encoded.to_vec()).expect("Failed to decode");

  let obj = Object::from(decoded);
  let payload_val = js_sys::Reflect::get(&obj, &wasm_bindgen::JsValue::from_str("payload"))
    .expect("Failed to get payload");
  let decoded_payload = Uint8Array::from(payload_val);

  assert_eq!(decoded_payload.to_vec(), payload);
  assert!(needs_chunking(payload.len()));
}
