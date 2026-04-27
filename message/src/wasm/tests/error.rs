use super::*;

// ========================================================================
// Error Path Tests — Invalid Inputs & Corrupted Payloads
// ========================================================================

#[wasm_bindgen_test]
fn test_decode_empty_input() {
  // Completely empty byte slice should fail
  let result = decode_message(&[]);
  assert!(result.is_err());
  let err_msg = result.unwrap_err().as_string().unwrap();
  assert!(
    err_msg.contains("Invalid message format"),
    "Expected 'Invalid message format', got: {err_msg}"
  );
}

#[wasm_bindgen_test]
fn test_decode_single_byte() {
  // Only 1 byte — too short for any valid frame
  let result = decode_message(&[0xBC]);
  assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_decode_header_only_no_payload() {
  // Valid magic + message type, but no payload bytes
  // decode_frame requires payload.len() > 0
  let frame_bytes = vec![0xBC, 0xBC, 0x01];
  let result = decode_message(&frame_bytes);
  assert!(result.is_err());
  let err_msg = result.unwrap_err().as_string().unwrap();
  assert!(
    err_msg.contains("Invalid message format"),
    "Expected 'Invalid message format' for header-only frame, got: {err_msg}"
  );
}

#[wasm_bindgen_test]
fn test_decode_wrong_first_magic_byte() {
  // First magic byte wrong, second correct
  let frame_bytes = vec![0x00, 0xBC, 0x01, 0xFF];
  let result = decode_message(&frame_bytes);
  assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_decode_wrong_second_magic_byte() {
  // First magic byte correct, second wrong
  let frame_bytes = vec![0xBC, 0x00, 0x01, 0xFF];
  let result = decode_message(&frame_bytes);
  assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_decode_both_magic_bytes_wrong() {
  // Both magic bytes wrong
  let frame_bytes = vec![0x00, 0x00, 0x01, 0xFF];
  let result = decode_message(&frame_bytes);
  assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_decode_swapped_magic_bytes() {
  // Magic bytes in wrong order (little-endian instead of big-endian)
  let frame_bytes = vec![0xBC, 0xBC, 0x01, 0xFF]; // This is actually correct
  let result = decode_message(&frame_bytes);
  assert!(result.is_ok());

  // Now try reversed: 0xCB, 0xCB
  let frame_bytes = vec![0xCB, 0xCB, 0x01, 0xFF];
  let result = decode_message(&frame_bytes);
  assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_encode_max_message_type() {
  // Message type 0xFF (maximum u8 value) should work
  let payload = vec![1, 2, 3];
  let result = encode_message(0xFF, &payload);
  assert!(result.is_ok());

  let encoded = result.unwrap().to_vec();
  assert_eq!(encoded[2], 0xFF);
}

#[wasm_bindgen_test]
fn test_encode_min_message_type() {
  // Message type 0x00 (minimum u8 value) should work
  let payload = vec![1, 2, 3];
  let result = encode_message(0x00, &payload);
  assert!(result.is_ok());

  let encoded = result.unwrap().to_vec();
  assert_eq!(encoded[2], 0x00);
}

#[wasm_bindgen_test]
fn test_encode_single_byte_payload() {
  // Minimum valid payload (1 byte)
  let payload = vec![0x42];
  let result = encode_message(0x01, &payload);
  assert!(result.is_ok());

  let encoded = result.unwrap().to_vec();
  assert_eq!(encoded.len(), 4); // 2 magic + 1 type + 1 payload
  assert_eq!(encoded[3], 0x42);
}

#[wasm_bindgen_test]
fn test_decode_corrupted_payload_bitcode_fails() {
  // Valid frame structure but payload is random garbage —
  // frame decode succeeds, but bitcode decode of the payload should fail.
  let mut frame_bytes = vec![0xBC, 0xBC, 0x00]; // magic + type=TokenAuth
  frame_bytes.extend_from_slice(&[0xFF, 0xFE, 0xFD, 0xFC, 0xFB]);

  // Frame-level decode should succeed (it doesn't validate payload content)
  let result = decode_message(&frame_bytes);
  assert!(result.is_ok());

  // But trying to bitcode-decode the payload as TokenAuth should fail
  let obj = Object::from(result.unwrap());
  let payload_val = js_sys::Reflect::get(&obj, &wasm_bindgen::JsValue::from_str("payload"))
    .expect("Failed to get payload");
  let decoded_payload = Uint8Array::from(payload_val).to_vec();

  use crate::signaling::TokenAuth;
  let bitcode_result = bitcode::decode::<TokenAuth>(&decoded_payload);
  assert!(
    bitcode_result.is_err(),
    "Corrupted payload should fail bitcode decode"
  );
}

#[wasm_bindgen_test]
fn test_decode_truncated_bitcode_payload() {
  // Encode a valid TokenAuth, then truncate the payload
  use crate::signaling::TokenAuth;
  let msg = TokenAuth {
    token: "a-long-enough-token-string".to_string(),
  };
  let payload = bitcode::encode(&msg);
  assert!(payload.len() > 2, "Payload should be > 2 bytes");

  // Encode into frame, then truncate
  let encoded = encode_message(0x00, &payload).unwrap().to_vec();
  let truncated = &encoded[..encoded.len() - 3]; // Remove last 3 bytes

  // Frame decode should still succeed (payload is non-empty)
  let result = decode_message(truncated);
  assert!(result.is_ok());

  // But bitcode decode should fail on the truncated payload
  let obj = Object::from(result.unwrap());
  let payload_val = js_sys::Reflect::get(&obj, &wasm_bindgen::JsValue::from_str("payload"))
    .expect("Failed to get payload");
  let decoded_payload = Uint8Array::from(payload_val).to_vec();

  let bitcode_result = bitcode::decode::<TokenAuth>(&decoded_payload);
  assert!(
    bitcode_result.is_err(),
    "Truncated payload should fail bitcode decode"
  );
}

#[wasm_bindgen_test]
fn test_decode_extra_bytes_after_payload() {
  // Valid frame with extra trailing bytes — decode_frame includes them in payload
  let payload = vec![1, 2, 3];
  let encoded = encode_message(0x01, &payload).unwrap().to_vec();

  // Append extra bytes
  let mut extended = encoded.clone();
  extended.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);

  let result = decode_message(&extended);
  assert!(result.is_ok());

  // The extra bytes become part of the payload
  let obj = Object::from(result.unwrap());
  let payload_val = js_sys::Reflect::get(&obj, &wasm_bindgen::JsValue::from_str("payload"))
    .expect("Failed to get payload");
  let decoded_payload = Uint8Array::from(payload_val).to_vec();
  assert_eq!(decoded_payload.len(), payload.len() + 4);
}

#[wasm_bindgen_test]
fn test_encode_decode_all_byte_values_payload() {
  // Payload containing every possible byte value (0x00..=0xFF)
  let payload: Vec<u8> = (0..=255).collect();
  let encoded = encode_message(0x42, &payload).unwrap();
  let decoded = decode_message(&encoded.to_vec()).unwrap();

  let obj = Object::from(decoded);
  let payload_val = js_sys::Reflect::get(&obj, &wasm_bindgen::JsValue::from_str("payload"))
    .expect("Failed to get payload");
  let decoded_payload = Uint8Array::from(payload_val).to_vec();
  assert_eq!(decoded_payload, payload);
}

// ========================================================================
// error_to_js_string Coverage Tests
// ========================================================================

#[wasm_bindgen_test]
fn test_error_to_js_string_invalid_format() {
  use crate::error::MessageError;
  let err = MessageError::InvalidFormat;
  let msg = error_to_js_string(&err);
  assert_eq!(msg, "Invalid message format");
}

#[wasm_bindgen_test]
fn test_error_to_js_string_serialization() {
  use crate::error::MessageError;
  let err = MessageError::Serialization("bitcode encode failed".to_string());
  let msg = error_to_js_string(&err);
  assert_eq!(msg, "Serialization error: bitcode encode failed");
}

#[wasm_bindgen_test]
fn test_error_to_js_string_deserialization() {
  use crate::error::MessageError;
  let err = MessageError::Deserialization("unexpected EOF".to_string());
  let msg = error_to_js_string(&err);
  assert_eq!(msg, "Deserialization error: unexpected EOF");
}

#[wasm_bindgen_test]
fn test_error_to_js_string_invalid_discriminator() {
  use crate::error::MessageError;
  let err = MessageError::InvalidDiscriminator(0xFE);
  let msg = error_to_js_string(&err);
  assert_eq!(msg, "Invalid message discriminator: 0xFE");
}

#[wasm_bindgen_test]
fn test_error_to_js_string_validation() {
  use crate::error::MessageError;
  let err = MessageError::Validation("Payload cannot be empty".to_string());
  let msg = error_to_js_string(&err);
  assert_eq!(msg, "Validation error: Payload cannot be empty");
}

#[wasm_bindgen_test]
fn test_error_to_js_string_discriminator_zero() {
  use crate::error::MessageError;
  let err = MessageError::InvalidDiscriminator(0x00);
  let msg = error_to_js_string(&err);
  assert_eq!(msg, "Invalid message discriminator: 0x00");
}

// ========================================================================
// Mismatched Message Type Decode Test
// ========================================================================

#[wasm_bindgen_test]
fn test_decode_wrong_message_type_bitcode_mismatch() {
  // Encode a ChatText (0x80) but try to decode as TokenAuth (0x00)
  use crate::datachannel::ChatText;
  use crate::signaling::TokenAuth;
  use crate::types::MessageId;

  let msg = ChatText {
    message_id: MessageId::new(),
    content: "Hello WASM".to_string(),
    reply_to: None,
    timestamp_nanos: 1_000_000_000,
  };
  let payload = bitcode::encode(&msg);
  let encoded = encode_message(0x80, &payload).unwrap().to_vec();

  // Frame decode succeeds
  let decoded = decode_message(&encoded).unwrap();
  let obj = Object::from(decoded);
  let payload_val = js_sys::Reflect::get(&obj, &wasm_bindgen::JsValue::from_str("payload"))
    .expect("Failed to get payload");
  let decoded_payload = Uint8Array::from(payload_val).to_vec();

  // Trying to decode ChatText payload as TokenAuth should fail or produce garbage
  let wrong_decode = bitcode::decode::<TokenAuth>(&decoded_payload);
  // This may or may not error depending on bitcode's behavior with mismatched types,
  // but the decoded value should NOT match the original message structure
  if let Ok(wrong_msg) = wrong_decode {
    // If bitcode happens to decode without error, the token should be garbage
    assert_ne!(
      wrong_msg.token, "Hello WASM",
      "Mismatched type decode should not produce original content"
    );
  }
  // If it errors, that's the expected behavior — test passes either way
}
