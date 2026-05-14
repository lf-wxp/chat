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

/// One ICE server entry pushed from the server to the client as part
/// of [`AuthSuccess`]. Mirrors the WebRTC `RTCIceServer` dictionary
/// (URL plus optional username / credential for TURN auth) so
/// deployments can configure intranet STUN/TURN endpoints purely via
/// environment variables (`STUN_TURN_SERVERS`) without a frontend
/// rebuild.
///
/// Wire format: a `Vec<IceServerSpec>` is appended to `AuthSuccess`.
/// Empty list means the client should keep its compiled-in default.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct IceServerSpec {
  /// `stun:` or `turn:` / `turns:` URL.
  pub url: String,
  /// Optional username (TURN only).
  pub username: Option<String>,
  /// Optional credential / shared secret (TURN only).
  pub credential: Option<String>,
}

/// Authentication success response.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct AuthSuccess {
  /// Authenticated user ID.
  pub user_id: UserId,
  /// Authenticated username.
  pub username: String,
  /// Display nickname (may differ from username).
  pub nickname: String,
  /// ICE servers (STUN/TURN) the client should pass to every
  /// `RTCPeerConnection`. Configured server-side via the
  /// `STUN_TURN_SERVERS` environment variable. An empty list tells
  /// the client to use its compiled-in default; this preserves
  /// backwards compatibility for older deployments where the env
  /// var is unset.
  #[serde(default)]
  pub ice_servers: Vec<IceServerSpec>,
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
