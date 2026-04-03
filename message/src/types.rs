//! Base type definitions

use serde::{Deserialize, Serialize};

/// Unique identifier type alias
pub type Id = String;

/// Timestamp (millisecond-precision Unix timestamp)
pub type Timestamp = i64;

/// Get the current timestamp in milliseconds
#[must_use]
pub fn now_timestamp() -> Timestamp {
  chrono::Utc::now().timestamp_millis()
}

/// Generate a unique ID
#[must_use]
pub fn gen_id() -> Id {
  nanoid::nanoid!()
}

/// Message state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageState {
  /// Sending
  Sending,
  /// Sent
  Sent,
  /// Failed to send
  Failed,
}

/// Media type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaType {
  /// Audio only
  Audio,
  /// Audio and video
  Video,
  /// Screen sharing
  Screen,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_message_state_serialize_roundtrip() {
    let states = vec![
      MessageState::Sending,
      MessageState::Sent,
      MessageState::Failed,
    ];
    for state in &states {
      let bytes = bitcode::serialize(state).expect("serialization failed");
      let decoded: MessageState = bitcode::deserialize(&bytes).expect("deserialization failed");
      assert_eq!(&decoded, state);
    }
  }

  #[test]
  fn test_media_type_serialize_roundtrip() {
    let types_list = vec![MediaType::Audio, MediaType::Video, MediaType::Screen];
    for mt in &types_list {
      let bytes = bitcode::serialize(mt).expect("serialization failed");
      let decoded: MediaType = bitcode::deserialize(&bytes).expect("deserialization failed");
      assert_eq!(&decoded, mt);
    }
  }
}
