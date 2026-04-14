//! Utility functions for WebSocket message encoding/decoding.

use axum::body::Bytes;
use axum::extract::ws::{Message, WebSocket};
use futures::SinkExt;
use futures::stream::SplitSink;
use message::error::{ErrorCategory, ErrorCode, ErrorModule};
use message::frame::{MessageFrame, encode_frame};
use message::signaling::SignalingMessage;

/// Encode a signaling message to binary frame.
///
/// Converts a `SignalingMessage` into a binary representation suitable for
/// WebSocket transmission. The encoding uses bitcode for efficient serialization.
///
/// # Arguments
///
/// * `msg` - The signaling message to encode.
///
/// # Returns
///
/// Returns `Ok(Vec<u8>)` containing the encoded binary frame,
/// or an error if serialization fails.
///
/// # Example
///
/// ```ignore
/// use message::signaling::SignalingMessage;
///
/// let msg = SignalingMessage::Ping(Ping::default());
/// let encoded = encode_signaling_message(&msg)?;
/// socket_tx.send(Message::Binary(Bytes::from(encoded))).await;
/// ```
pub fn encode_signaling_message(
  msg: &SignalingMessage,
) -> Result<Vec<u8>, message::error::MessageError> {
  let discriminator = msg.discriminator();
  let payload = bitcode::encode(msg);
  let frame = MessageFrame::new(discriminator, payload);
  encode_frame(&frame)
}

/// Decode a signaling message from binary frame.
///
/// Reconstructs a `SignalingMessage` from its binary frame representation.
/// The decoding uses bitcode for efficient deserialization.
///
/// # Arguments
///
/// * `frame` - The message frame containing the discriminator and payload.
///
/// # Returns
///
/// Returns `Ok(SignalingMessage)` on success, or an error if deserialization fails.
///
/// # Errors
///
/// Returns `MessageError::Deserialization` if the payload cannot be decoded.
pub fn decode_signaling_message(
  frame: &MessageFrame,
) -> Result<SignalingMessage, message::error::MessageError> {
  bitcode::decode(&frame.payload).map_err(|e| {
    message::error::MessageError::Deserialization(format!(
      "Failed to decode signaling message: {e}"
    ))
  })
}

/// Send an error response to the client.
///
/// Constructs and sends an `ErrorResponse` message through the WebSocket connection.
/// The error includes a code, message, and optional i18n key for client-side localization.
///
/// # Arguments
///
/// * `socket_tx` - The WebSocket sender to use for sending the error.
/// * `code` - The error code string (e.g., "ROM701", "SIG001").
/// * `message` - Human-readable error message.
/// * `i18n_key` - Optional internationalization key for client-side lookup.
pub async fn send_error_response(
  socket_tx: &mut SplitSink<WebSocket, Message>,
  code: &str,
  message: &str,
  i18n_key: Option<&str>,
) {
  // Parse error code string to ErrorCode
  let error_code = parse_error_code(code);

  let error_msg = SignalingMessage::ErrorResponse(message::ErrorResponse {
    code: error_code,
    message: message.to_string(),
    i18n_key: i18n_key.unwrap_or("error.unknown").to_string(),
    details: std::collections::HashMap::new(),
    timestamp_nanos: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
    trace_id: uuid::Uuid::new_v4().to_string(),
  });

  if let Ok(encoded) = encode_signaling_message(&error_msg) {
    let _ = socket_tx.send(Message::Binary(Bytes::from(encoded))).await;
  }
}

/// Parse error code string (e.g. "ROM701", "SIG001") to ErrorCode enum.
///
/// The numeric part after the module prefix is treated as a single integer N,
/// then stored as `category = Network(0)` and `sequence = N`. This ensures
/// `to_code_string()` reproduces the original string via `category*100 + sequence`.
fn parse_error_code(code: &str) -> ErrorCode {
  // Detect module prefix length: most are 3 chars, AUTH is 4 chars.
  let (module, prefix_len) = if code.starts_with("AUTH") {
    (ErrorModule::Auth, 4)
  } else if code.len() >= 3 {
    let module = match &code[0..3] {
      "SIG" => ErrorModule::Sig,
      "CHT" => ErrorModule::Cht,
      "ROM" => ErrorModule::Rom,
      "E2E" => ErrorModule::E2e,
      "FIL" => ErrorModule::Fil,
      "THR" => ErrorModule::Thr,
      "PST" => ErrorModule::Pst,
      "SYS" => ErrorModule::Sys,
      "AV" => return parse_with_prefix(code, ErrorModule::Av, 2),
      _ => ErrorModule::Sys,
    };
    (module, 3)
  } else {
    return ErrorCode::new(ErrorModule::Sys, ErrorCategory::Client, 0);
  };

  parse_with_prefix(code, module, prefix_len)
}

/// Helper: parse the numeric part after a known module prefix.
fn parse_with_prefix(code: &str, module: ErrorModule, prefix_len: usize) -> ErrorCode {
  let numeric_str = &code[prefix_len..];
  let n = numeric_str.parse::<u16>().unwrap_or(0);
  // Store the entire numeric part as category=Network(0), sequence=N.
  // to_code_string() computes category*100 + sequence = 0*100 + N = N,
  // then formats as "{module}{N:03}" which reproduces the original string.
  ErrorCode::new(module, ErrorCategory::Network, n)
}

#[cfg(test)]
mod tests;
