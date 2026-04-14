//! Call signaling messages.

use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::types::{MediaType, RoomId};

/// Call invitation.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct CallInvite {
  /// Room ID for the call.
  pub room_id: RoomId,
  /// Media type (Audio/Video/ScreenShare).
  pub media_type: MediaType,
}

/// Accept call.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct CallAccept {
  /// Room ID for the call.
  pub room_id: RoomId,
}

/// Decline call.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct CallDecline {
  /// Room ID for the call.
  pub room_id: RoomId,
}

/// End call.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct CallEnd {
  /// Room ID for the call.
  pub room_id: RoomId,
}
