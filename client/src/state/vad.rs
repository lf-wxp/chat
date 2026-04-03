//! VAD (Voice Activity Detection) speaker state

use std::collections::{HashMap, HashSet};

/// VAD speaker detection state
#[derive(Debug, Clone, Default)]
pub struct VadState {
  /// Set of user IDs currently speaking
  pub speaking_users: HashSet<String>,
  /// Volume level per user (0-100)
  pub volume_levels: HashMap<String, f64>,
}

impl VadState {
  /// Check if the specified user is currently speaking
  pub fn is_speaking(&self, user_id: &str) -> bool {
    self.speaking_users.contains(user_id)
  }

  /// Get volume level for the specified user (0-100), returns 0 for unknown users
  pub fn volume(&self, user_id: &str) -> f64 {
    self.volume_levels.get(user_id).copied().unwrap_or(0.0)
  }
}
