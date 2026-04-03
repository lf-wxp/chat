//! User authentication state

use message::signal::UserStatus;

/// Current user authentication information
#[derive(Debug, Clone, Default)]
pub struct UserState {
  /// Whether the user is authenticated
  pub authenticated: bool,
  /// User ID
  pub user_id: String,
  /// Username
  pub username: String,
  /// JWT Token
  pub token: String,
  /// Online status
  pub status: UserStatus,
}
