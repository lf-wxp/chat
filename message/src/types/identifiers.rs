//! Identifier types for the WebRTC Chat Application.
//!
//! This module defines unique identifier types used throughout the application,
//! including `UserId`, `RoomId`, `MessageId`, and `TransferId`.

use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for a user.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode)]
pub struct UserId(pub Uuid);

impl UserId {
  /// Create a new random `UserId`.
  #[must_use]
  pub fn new() -> Self {
    Self(Uuid::new_v4())
  }

  /// Create a `UserId` from a `Uuid`.
  #[must_use]
  pub const fn from_uuid(uuid: Uuid) -> Self {
    Self(uuid)
  }

  /// Get the inner `Uuid`.
  #[must_use]
  pub const fn as_uuid(&self) -> &Uuid {
    &self.0
  }
}

impl Default for UserId {
  fn default() -> Self {
    Self::new()
  }
}

impl fmt::Display for UserId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

/// Unique identifier for a room.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode)]
pub struct RoomId(pub Uuid);

impl RoomId {
  /// Create a new random `RoomId`.
  #[must_use]
  pub fn new() -> Self {
    Self(Uuid::new_v4())
  }

  /// Create a `RoomId` from a `Uuid`.
  #[must_use]
  pub const fn from_uuid(uuid: Uuid) -> Self {
    Self(uuid)
  }
}

impl Default for RoomId {
  fn default() -> Self {
    Self::new()
  }
}

impl fmt::Display for RoomId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

/// Unique identifier for a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode)]
pub struct MessageId(pub Uuid);

impl MessageId {
  /// Create a new random `MessageId`.
  #[must_use]
  pub fn new() -> Self {
    Self(Uuid::new_v4())
  }

  /// Create a `MessageId` from a `Uuid`.
  #[must_use]
  pub const fn from_uuid(uuid: Uuid) -> Self {
    Self(uuid)
  }

  /// Create a nil (all-zeros) `MessageId`.
  #[must_use]
  pub const fn nil() -> Self {
    Self(Uuid::nil())
  }

  /// Get the inner `Uuid`.
  #[must_use]
  pub const fn as_uuid(&self) -> &Uuid {
    &self.0
  }
}

impl Default for MessageId {
  fn default() -> Self {
    Self::new()
  }
}

impl fmt::Display for MessageId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

/// Unique identifier for a file transfer.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode)]
pub struct TransferId(pub Uuid);

impl TransferId {
  /// Create a new random `TransferId`.
  #[must_use]
  pub fn new() -> Self {
    Self(Uuid::new_v4())
  }
}

impl Default for TransferId {
  fn default() -> Self {
    Self::new()
  }
}

impl fmt::Display for TransferId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}
