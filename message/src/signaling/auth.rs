//! Connection & authentication signaling messages.

use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::types::UserId;

/// JWT authentication on WebSocket connect / page refresh.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct TokenAuth {
  /// JWT token for authentication.
  pub token: String,
}

/// Authentication success response.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct AuthSuccess {
  /// Authenticated user ID.
  pub user_id: UserId,
  /// Authenticated username.
  pub username: String,
}

/// Authentication failure response.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct AuthFailure {
  /// Failure reason.
  pub reason: String,
}

/// Active logout notification.
#[derive(Debug, Clone, Default, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct UserLogout {}

/// Heartbeat ping.
#[derive(Debug, Clone, Default, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct Ping {}

/// Heartbeat pong.
#[derive(Debug, Clone, Default, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct Pong {}

/// Session invalidated by another device login.
/// Sent to old connection when user logs in from a new device.
#[derive(Debug, Clone, Default, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct SessionInvalidated {}
