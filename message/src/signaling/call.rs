//! Call signaling messages.
//!
//! All four message types carry a `from: UserId` field identifying the
//! sender. Clients SHOULD populate `from` with their own user id when
//! emitting these messages; the signaling server MUST overwrite it with
//! the authenticated session's user id before broadcasting, so peers
//! always observe a trusted sender (Req 7.4 — incoming call modal needs
//! the caller's identity to render the avatar/nickname).

use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::UserId;
use crate::types::{MediaType, RoomId};

/// Call invitation.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct CallInvite {
  /// Sender of the invite. Filled by the server with the authenticated
  /// user id before broadcasting; clients MAY pre-populate it with
  /// their own id but the field is authoritative only after server
  /// rewriting.
  pub from: UserId,
  /// Room ID for the call.
  pub room_id: RoomId,
  /// Media type (Audio/Video/ScreenShare).
  pub media_type: MediaType,
}

/// Accept call.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct CallAccept {
  /// Sender of the accept. See [`CallInvite::from`] for trust semantics.
  pub from: UserId,
  /// Room ID for the call.
  pub room_id: RoomId,
}

/// Decline call.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct CallDecline {
  /// Sender of the decline. See [`CallInvite::from`] for trust semantics.
  pub from: UserId,
  /// Room ID for the call.
  pub room_id: RoomId,
}

/// End call.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct CallEnd {
  /// Sender of the end notification. See [`CallInvite::from`] for trust
  /// semantics.
  pub from: UserId,
  /// Room ID for the call.
  pub room_id: RoomId,
}
