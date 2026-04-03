//! Online users list state

use message::signal::OnlineUser;

/// Online users list state
#[derive(Debug, Clone, Default)]
pub struct OnlineUsersState {
  /// Online users list
  pub users: Vec<OnlineUser>,
  /// Search filter keyword
  pub search_query: String,
}
