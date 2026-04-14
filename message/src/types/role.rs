//! Room role types for the WebRTC Chat Application.
//!
//! This module defines the `RoomRole` enum with ordering support.

use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Room member role.
#[derive(
  Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode,
)]
#[serde(rename_all = "snake_case")]
pub enum RoomRole {
  /// Room owner with full control
  Owner,
  /// Room admin with moderation powers
  Admin,
  /// Regular room member
  #[default]
  Member,
}

impl fmt::Display for RoomRole {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Owner => write!(f, "Owner"),
      Self::Admin => write!(f, "Admin"),
      Self::Member => write!(f, "Member"),
    }
  }
}

impl PartialOrd for RoomRole {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for RoomRole {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    let rank = |role: &Self| match role {
      Self::Owner => 2,
      Self::Admin => 1,
      Self::Member => 0,
    };
    rank(self).cmp(&rank(other))
  }
}
