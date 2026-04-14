//! Connection invitation signaling messages.

use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::types::UserId;

/// Connection invitation.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct ConnectionInvite {
  /// Inviter user ID.
  pub from: UserId,
  /// Target user ID.
  pub to: UserId,
  /// Optional invitation note.
  pub note: Option<String>,
}

/// Invitation accepted.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct InviteAccepted {
  /// The user who accepts the invitation (invitee / sender of this message).
  pub from: UserId,
  /// The original inviter who should receive the acceptance.
  pub to: UserId,
}

/// Invitation declined.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct InviteDeclined {
  /// The user who declines the invitation (invitee / sender of this message).
  pub from: UserId,
  /// The original inviter who should receive the decline.
  pub to: UserId,
}

/// Invitation timed out.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct InviteTimeout {
  /// The user who reports the timeout (invitee / sender of this message).
  pub from: UserId,
  /// The original inviter who should receive the timeout notification.
  pub to: UserId,
}

/// Multi-user invitation.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct MultiInvite {
  /// Inviter user ID.
  pub from: UserId,
  /// Target user IDs.
  pub targets: Vec<UserId>,
}
