//! Authentication signaling messages.

use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

// Re-export UserId from types
use crate::types::UserId;

// ---------------------------------------------------------
// Token authentication message types for signaling protocol
// ---------------------------------------------------------

/// JWT authentication message.
///
/// Client sends this immediately after WebSocket connection is established
/// to authenticate the session. The server responds with `AuthSuccess` or `AuthFailure`.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct TokenAuth {
  /// JWT token string.
  pub token: String,
}

/// Authentication success response.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct AuthSuccess {
  /// Authenticated user ID.
  pub user_id: UserId,
  /// Username.
  pub username: String,
  /// Display nickname.
  pub nickname: String,
  /// ICE server configuration for WebRTC.
  pub ice_servers: Vec<IceServerSpec>,
  /// Avatar URL (data URL or CDN URL).
  pub avatar_url: Option<String>,
}

/// Authentication failure response.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct AuthFailure {
  /// Reason for auth failure.
  pub reason: String,
}

/// User logout message.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize, Default)]
pub struct UserLogout;

/// Session invalidated by another device login.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize, Default)]
pub struct SessionInvalidated;

// ---------------------------------------------------------
// Heartbeat messages
// ---------------------------------------------------------

/// Heartbeat ping message.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize, Default)]
pub struct Ping;

/// Heartbeat pong message.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize, Default)]
pub struct Pong;

// ---------------------------------------------------------
// ICE server specification
// ---------------------------------------------------------

/// ICE server configuration.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct IceServerSpec {
  /// Server URL (e.g., "stun:stun.example.com").
  pub url: String,
  /// Optional username for TURN server.
  pub username: Option<String>,
  /// Optional credential for TURN server.
  pub credential: Option<String>,
}
