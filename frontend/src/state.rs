//! Global application state.

use leptos::prelude::*;
use message::UserId;

/// Authentication state.
#[derive(Debug, Clone)]
pub struct AuthState {
  /// User ID
  pub user_id: UserId,
  /// JWT token
  pub token: String,
}

/// Global application state.
#[derive(Debug, Clone, Copy)]
pub struct AppState {
  /// Authentication state
  pub auth: RwSignal<Option<AuthState>>,
  /// Online users list
  pub online_users: RwSignal<Vec<message::UserId>>,
  /// WebSocket connection state
  pub connected: RwSignal<bool>,
}

impl AppState {
  /// Create new application state.
  #[must_use]
  pub fn new() -> Self {
    Self {
      auth: RwSignal::new(None),
      online_users: RwSignal::new(Vec::new()),
      connected: RwSignal::new(false),
    }
  }

  /// Check if user is authenticated.
  #[must_use]
  pub fn is_authenticated(&self) -> bool {
    self.auth.get().is_some()
  }

  /// Get current user ID.
  #[must_use]
  pub fn current_user_id(&self) -> Option<UserId> {
    self.auth.get().map(|state| state.user_id)
  }
}

impl Default for AppState {
  fn default() -> Self {
    Self::new()
  }
}
