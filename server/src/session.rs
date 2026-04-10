//! Session management module.

use message::UserId;

/// User session information.
#[derive(Debug, Clone)]
pub struct Session {
  /// User ID
  pub user_id: UserId,
  /// Connection timestamp
  pub connected_at: chrono::DateTime<chrono::Utc>,
}

impl Session {
  /// Create a new session.
  #[must_use]
  pub fn new(user_id: UserId) -> Self {
    Self {
      user_id,
      connected_at: chrono::Utc::now(),
    }
  }
}
