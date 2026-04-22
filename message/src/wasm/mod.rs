//! WASM bindings for message crate.
//!
//! This module provides JavaScript-friendly APIs for encoding and decoding
//! message frames in WebAssembly context using wasm-bindgen.
//!
//! ## Features
//!
//! - Zero-copy `ArrayBuffer` ↔ `Vec<u8>` conversion
//! - JavaScript-friendly error handling
//! - Frame-level encoding and decoding
//!
//! ## Usage
//!
//! ```javascript
//! import * as message from 'message';
//!
//! // Initialize the WASM module (call once on load)
//! message.initWasm();
//!
//! // Encode a message frame
//! const messageType = 0x01;  // SignalingMessage discriminator
//! const payload = new Uint8Array([/* bitcode-encoded message */]);
//! const frame = message.encodeMessage(messageType, payload);
//!
//! // Decode a received frame
//! const decoded = message.decodeMessage(receivedBytes);
//! console.log(decoded.messageType);  // 0x01
//! console.log(decoded.payload);      // Uint8Array
//! ```

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use js_sys::{ArrayBuffer, Uint8Array};

#[cfg(target_arch = "wasm32")]
use crate::error::MessageError;
#[cfg(target_arch = "wasm32")]
use crate::frame::{MessageFrame, decode_frame, encode_frame};

// =============================================================================
// ArrayBuffer Conversion Utilities
// =============================================================================

/// Convert a `Vec<u8>` to JavaScript `ArrayBuffer`.
///
/// This performs a zero-copy conversion where possible.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn vec_to_array_buffer(vec: &[u8]) -> ArrayBuffer {
  let array = Uint8Array::new_with_length(vec.len().try_into().unwrap_or(0));
  array.copy_from(vec);
  array.buffer()
}

/// Convert JavaScript `ArrayBuffer` to `Vec<u8>`.
///
/// This performs a copy from the JavaScript memory to Rust memory.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn array_buffer_to_vec(buffer: &ArrayBuffer) -> Vec<u8> {
  let array = Uint8Array::new(buffer);
  array.to_vec()
}

/// Convert JavaScript `Uint8Array` to `Vec<u8>`.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn uint8_array_to_vec(array: &Uint8Array) -> Vec<u8> {
  array.to_vec()
}

/// Convert `Vec<u8>` to JavaScript `Uint8Array`.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn vec_to_uint8_array(vec: &[u8]) -> Uint8Array {
  let array = Uint8Array::new_with_length(vec.len().try_into().unwrap_or(0));
  array.copy_from(vec);
  array
}

// =============================================================================
// Error Conversion
// =============================================================================

/// Convert a `MessageError` to a JavaScript-friendly error message.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn error_to_js_string(err: &MessageError) -> String {
  match err {
    MessageError::InvalidFormat => "Invalid message format".to_string(),
    MessageError::Serialization(msg) => format!("Serialization error: {msg}"),
    MessageError::Deserialization(msg) => format!("Deserialization error: {msg}"),
    MessageError::InvalidDiscriminator(disc) => {
      format!("Invalid message discriminator: 0x{disc:02X}")
    }
    MessageError::Validation(msg) => format!("Validation error: {msg}"),
  }
}

// =============================================================================
// WASM-bindgen Public API
// =============================================================================

/// Encode a message frame from type and payload.
///
/// # Arguments
/// * `message_type` - The message type discriminator byte
/// * `payload` - The message payload bytes (typically bitcode-encoded)
///
/// # Returns
/// A `Uint8Array` containing the complete frame:
/// - 2 bytes: magic number (0xBCBC)
/// - 1 byte: message type
/// - N bytes: payload
///
/// # Errors
/// Returns a JavaScript error if encoding fails.
///
/// # Example
/// ```javascript
/// const payload = bitcodeEncode(myMessage);
/// const frame = message.encodeMessage(0x01, payload);
/// ```
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = encodeMessage)]
pub fn encode_message(message_type: u8, payload: &[u8]) -> Result<Uint8Array, JsValue> {
  let frame = MessageFrame::new(message_type, payload.to_vec());

  encode_frame(&frame)
    .map(|v| vec_to_uint8_array(&v))
    .map_err(|e| JsValue::from_str(&error_to_js_string(&e)))
}

/// Decode a message frame into type and payload.
///
/// # Arguments
/// * `bytes` - A byte array containing the encoded frame
///
/// # Returns
/// An object with `messageType` (number) and `payload` (`Uint8Array`) properties.
///
/// # Errors
/// Returns a JavaScript error if:
/// - The frame is too short (< 3 bytes)
/// - The magic number is invalid
/// - The payload length doesn't match
///
/// # Example
/// ```javascript
/// const decoded = message.decodeMessage(receivedBytes);
/// console.log(decoded.messageType);  // 0x01
/// const msg = bitcodeDecode(decoded.payload, messageType);
/// ```
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = decodeMessage)]
pub fn decode_message(bytes: &[u8]) -> Result<JsValue, JsValue> {
  let frame = decode_frame(bytes).map_err(|e| JsValue::from_str(&error_to_js_string(&e)))?;

  // Create result object
  let result = js_sys::Object::new();
  js_sys::Reflect::set(
    &result,
    &JsValue::from_str("messageType"),
    &JsValue::from(frame.message_type),
  )
  .map_err(|e| JsValue::from_str(&format!("Failed to set messageType: {e:?}")))?;

  let payload = vec_to_uint8_array(&frame.payload);
  js_sys::Reflect::set(
    &result,
    &JsValue::from_str("payload"),
    &JsValue::from(payload),
  )
  .map_err(|e| JsValue::from_str(&format!("Failed to set payload: {e:?}")))?;

  Ok(result.into())
}

/// Encode a message from `ArrayBuffer` input.
///
/// This is a convenience function that accepts an `ArrayBuffer` instead of a byte slice.
///
/// # Arguments
/// * `message_type` - The message type discriminator byte
/// * `buffer` - An `ArrayBuffer` containing the message payload
///
/// # Returns
/// A `Uint8Array` containing the complete frame.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = encodeMessageFromBuffer)]
pub fn encode_message_from_buffer(
  message_type: u8,
  buffer: &ArrayBuffer,
) -> Result<Uint8Array, JsValue> {
  let payload = array_buffer_to_vec(buffer);
  encode_message(message_type, &payload)
}

/// Decode a message from `ArrayBuffer` input.
///
/// This is a convenience function that accepts an `ArrayBuffer` instead of a byte slice.
///
/// # Arguments
/// * `buffer` - An `ArrayBuffer` containing the encoded frame
///
/// # Returns
/// An object with `messageType` and `payload` properties.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = decodeMessageFromBuffer)]
pub fn decode_message_from_buffer(buffer: &ArrayBuffer) -> Result<JsValue, JsValue> {
  let bytes = array_buffer_to_vec(buffer);
  decode_message(&bytes)
}

// =============================================================================
// Constants
// =============================================================================

/// Get the magic number used in frame headers.
///
/// Returns: 0xBCBC (48316 in decimal)
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = getMagicNumber)]
#[must_use]
pub fn get_magic_number() -> u16 {
  crate::frame::MAGIC_NUMBER
}

/// Get the magic number as bytes (big-endian).
///
/// Returns: `[0xBC, 0xBC]`
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = getMagicNumberBytes)]
#[must_use]
pub fn get_magic_number_bytes() -> Vec<u8> {
  crate::frame::MAGIC_NUMBER_BYTES.to_vec()
}

/// Get the header size in bytes.
///
/// The header consists of:
/// - 2 bytes: magic number
/// - 1 byte: message type
///
/// Total: 3 bytes
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = getHeaderSize)]
#[must_use]
pub fn get_header_size() -> usize {
  3 // 2 bytes magic + 1 byte type
}

// =============================================================================
// Chunking Support
// =============================================================================

/// Check if a payload needs chunking.
///
/// Messages larger than the chunking threshold (64KB) should be split
/// into multiple chunks for reliable transmission.
///
/// # Arguments
/// * `payload_len` - The length of the payload in bytes
///
/// # Returns
/// `true` if the message needs to be chunked.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = needsChunking)]
#[must_use]
pub fn needs_chunking(payload_len: usize) -> bool {
  payload_len > crate::frame::CHUNKING_THRESHOLD
}

/// Get the maximum chunk size in bytes.
///
/// Returns: 65536 (64KB)
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = getMaxChunkSize)]
#[must_use]
pub fn get_max_chunk_size() -> usize {
  crate::frame::MAX_CHUNK_SIZE
}

/// Get the chunking threshold in bytes.
///
/// Messages larger than this threshold should be chunked.
/// Returns: 65536 (64KB)
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = getChunkingThreshold)]
#[must_use]
pub fn get_chunking_threshold() -> usize {
  crate::frame::CHUNKING_THRESHOLD
}

/// Calculate the number of chunks needed for a payload.
///
/// # Arguments
/// * `payload_len` - The length of the payload in bytes
///
/// # Returns
/// The number of chunks needed (minimum 1).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = calculateChunkCount)]
#[must_use]
pub fn calculate_chunk_count(payload_len: usize) -> usize {
  if payload_len == 0 {
    return 1;
  }
  payload_len.div_ceil(crate::frame::MAX_CHUNK_SIZE)
}

// =============================================================================
// WASM Initialization
// =============================================================================

/// Initialize the WASM module.
///
/// This should be called when the WASM module is loaded.
/// It sets up the panic hook for better error messages in the browser console.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = initWasm)]
pub fn init_wasm() {
  // Set up panic hook for better error messages in browser console
  #[cfg(feature = "console_error_panic_hook")]
  console_error_panic_hook::set_once();
}

// =============================================================================
// WASM Tests
// =============================================================================

#[cfg(target_arch = "wasm32")]
#[cfg(test)]
mod tests;
