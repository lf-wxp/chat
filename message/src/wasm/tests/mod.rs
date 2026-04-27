//! WASM frame encode/decode tests.
//!
//! Tests are organized by functionality:
//! - `frame`: Basic frame encoding, decoding, and roundtrip tests
//! - `signaling`: Signaling message roundtrip through WASM frame pipeline
//! - `datachannel`: DataChannel message roundtrip through WASM frame pipeline
//! - `error`: Error path and invalid input tests
//! - `conversion`: ArrayBuffer conversion, chunking, and boundary tests

mod conversion;
mod datachannel;
mod error;
mod frame;
mod signaling;

// Re-export all necessary types for test submodules
pub(super) use super::{
  calculate_chunk_count, decode_message, encode_message, error_to_js_string,
  get_chunking_threshold, get_header_size, get_magic_number, get_magic_number_bytes,
  get_max_chunk_size, needs_chunking,
};

#[cfg(target_arch = "wasm32")]
pub(super) use super::{
  array_buffer_to_vec, decode_message_from_buffer, encode_message_from_buffer, uint8_array_to_vec,
  vec_to_array_buffer, vec_to_uint8_array,
};

pub(super) use js_sys::{Object, Uint8Array};
pub(super) use wasm_bindgen_test::*;

/// Helper: encode a bitcode-serializable message through the WASM frame
/// pipeline and verify it decodes back with matching type and payload.
///
/// For messages whose bitcode encoding yields an empty payload (e.g. unit
/// structs like Ping/Pong), the frame-level roundtrip is skipped because
/// `encode_message` rejects empty payloads by design. In that case we
/// only verify bitcode encode→decode consistency, which matches the
/// approach used in the non-WASM signaling tests.
pub(super) fn roundtrip_signaling<
  T: bitcode::Encode + for<'a> bitcode::Decode<'a> + PartialEq + std::fmt::Debug,
>(
  msg_type: u8,
  msg: &T,
) {
  let payload = bitcode::encode(msg);

  if payload.is_empty() {
    // Empty-payload messages (unit structs) cannot go through the frame
    // encoder which rejects empty payloads. Verify bitcode roundtrip only.
    let decoded_msg: T = bitcode::decode(&payload).expect("Failed to decode payload");
    assert_eq!(*msg, decoded_msg);
    return;
  }

  let encoded = encode_message(msg_type, &payload).expect("Failed to encode");
  let decoded = decode_message(&encoded.to_vec()).expect("Failed to decode");

  let obj = Object::from(decoded);
  let decoded_type = js_sys::Reflect::get(&obj, &wasm_bindgen::JsValue::from_str("messageType"))
    .expect("Failed to get messageType")
    .as_f64()
    .expect("messageType is not a number") as u8;
  assert_eq!(decoded_type, msg_type);

  let payload_val = js_sys::Reflect::get(&obj, &wasm_bindgen::JsValue::from_str("payload"))
    .expect("Failed to get payload");
  let decoded_payload = Uint8Array::from(payload_val).to_vec();
  let decoded_msg: T = bitcode::decode(&decoded_payload).expect("Failed to decode payload");
  assert_eq!(*msg, decoded_msg);
}

/// Helper: encode a bitcode-serializable DataChannel message through the
/// WASM frame pipeline. Same logic as `roundtrip_signaling` but using the
/// DataChannel discriminator namespace (0x80–0xB3).
pub(super) fn roundtrip_datachannel<
  T: bitcode::Encode + for<'a> bitcode::Decode<'a> + PartialEq + std::fmt::Debug,
>(
  msg_type: u8,
  msg: &T,
) {
  let payload = bitcode::encode(msg);

  if payload.is_empty() {
    let decoded_msg: T = bitcode::decode(&payload).expect("Failed to decode payload");
    assert_eq!(*msg, decoded_msg);
    return;
  }

  let encoded = encode_message(msg_type, &payload).expect("Failed to encode");
  let decoded = decode_message(&encoded.to_vec()).expect("Failed to decode");

  let obj = Object::from(decoded);
  let decoded_type = js_sys::Reflect::get(&obj, &wasm_bindgen::JsValue::from_str("messageType"))
    .expect("Failed to get messageType")
    .as_f64()
    .expect("messageType is not a number") as u8;
  assert_eq!(decoded_type, msg_type);

  let payload_val = js_sys::Reflect::get(&obj, &wasm_bindgen::JsValue::from_str("payload"))
    .expect("Failed to get payload");
  let decoded_payload = Uint8Array::from(payload_val).to_vec();
  let decoded_msg: T = bitcode::decode(&decoded_payload).expect("Failed to decode payload");
  assert_eq!(*msg, decoded_msg);
}
