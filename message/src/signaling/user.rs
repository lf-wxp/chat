//! User discovery & status signaling messages.

use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::types::{UserId, UserInfo, UserStatus};

/// Full/incremental online user list update.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct UserListUpdate {
  /// List of online users.
  pub users: Vec<UserInfo>,
}

/// User status/signature change broadcast.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct UserStatusChange {
  /// User ID.
  pub user_id: UserId,
  /// New status.
  pub status: UserStatus,
  /// Optional signature/message.
  pub signature: Option<String>,
}
