//! Server global state

use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::mpsc;

use crate::auth::UserSession;
use crate::filter_stats::FilterStatsManager;
use crate::room::RoomManager;
use message::signal::InviteType;
use message::types::Id;

/// Application global state (shared via Axum State)
#[derive(Clone)]
pub struct AppState {
  inner: Arc<AppStateInner>,
}

/// Internal state
pub struct AppStateInner {
  /// Online WebSocket connections: user_id -> sender
  pub connections: DashMap<String, ConnectionHandle>,
  /// User sessions (in-memory, no persistence): user_id -> session
  pub sessions: DashMap<String, UserSession>,
  /// Username to user ID mapping (for checking username uniqueness)
  pub username_map: DashMap<String, String>,
  /// Room manager
  pub rooms: RoomManager,
  /// Offline invite staging: user_id -> pending invites
  pub pending_invites: DashMap<String, Vec<message::signal::SignalMessage>>,
  /// Invite link storage: code -> InviteLinkEntry
  pub invite_links: DashMap<String, InviteLinkEntry>,
  /// Active invite tracking (deduplication): "from_id->to_id" -> expiration timestamp (ms)
  pub active_invites: DashMap<String, i64>,
  /// Invite rate limiting: user_id -> list of invite timestamps in the last minute (ms)
  pub invite_rate_limits: DashMap<String, Vec<i64>>,
  /// JWT secret key
  pub jwt_secret: String,
  /// STUN/TURN server configuration
  pub ice_servers: Vec<String>,
  /// Sensitive word filter statistics manager
  pub filter_stats: FilterStatsManager,
}

/// WebSocket connection handle
pub struct ConnectionHandle {
  pub user_id: String,
  pub username: String,
  pub tx: mpsc::UnboundedSender<axum::extract::ws::Message>,
}

impl AppState {
  /// Create new application state
  #[must_use]
  pub fn new() -> Self {
    let jwt_secret =
      std::env::var("JWT_SECRET").unwrap_or_else(|_| "chat-app-secret-key-2024".to_string());

    let ice_servers = std::env::var("ICE_SERVERS")
      .unwrap_or_else(|_| "stun:stun.l.google.com:19302".to_string())
      .split(',')
      .map(String::from)
      .collect();

    Self {
      inner: Arc::new(AppStateInner {
        connections: DashMap::new(),
        sessions: DashMap::new(),
        username_map: DashMap::new(),
        rooms: RoomManager::new(),
        pending_invites: DashMap::new(),
        invite_links: DashMap::new(),
        active_invites: DashMap::new(),
        invite_rate_limits: DashMap::new(),
        jwt_secret,
        ice_servers,
        filter_stats: FilterStatsManager::default(),
      }),
    }
  }

  /// Get inner state reference
  pub fn inner(&self) -> &AppStateInner {
    &self.inner
  }
}

impl Default for AppState {
  fn default() -> Self {
    Self::new()
  }
}

/// Invite link entry
pub struct InviteLinkEntry {
  /// Creator user ID
  pub creator_id: Id,
  /// Creator username
  pub creator_username: String,
  /// Invite type
  pub invite_type: InviteType,
  /// Target room ID (only for Room type invites)
  pub room_id: Option<Id>,
  /// Expiration timestamp (milliseconds)
  pub expires_at: i64,
  /// Whether already used
  pub used: bool,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_app_state_new_default_values() {
    let state = AppState::new();
    // Initial state should have no connections, sessions, or rooms
    assert_eq!(state.inner().connections.len(), 0);
    assert_eq!(state.inner().sessions.len(), 0);
    assert_eq!(state.inner().username_map.len(), 0);
    assert_eq!(state.inner().pending_invites.len(), 0);
    assert_eq!(state.inner().invite_links.len(), 0);
    assert_eq!(state.inner().active_invites.len(), 0);
  }

  #[test]
  fn test_app_state_default_jwt_secret() {
    let state = AppState::new();
    // Use default secret when environment variable is not set
    assert!(!state.inner().jwt_secret.is_empty());
  }

  #[test]
  fn test_app_state_default_ice_servers() {
    let state = AppState::new();
    // Default should include STUN server
    assert!(!state.inner().ice_servers.is_empty());
    assert!(state.inner().ice_servers[0].contains("stun"));
  }

  #[test]
  fn test_app_state_clone_shares_data() {
    let state = AppState::new();
    let state_clone = state.clone();

    // Data inserted on clone is visible on original state
    state_clone
      .inner()
      .username_map
      .insert("alice".to_string(), "user-1".to_string());
    assert!(state.inner().username_map.contains_key("alice"));
  }

  #[test]
  fn test_app_state_default_trait() {
    // Default implementation should be consistent with new()
    let state = AppState::default();
    assert_eq!(state.inner().connections.len(), 0);
  }

  #[test]
  fn test_invite_link_entry_storage() {
    let state = AppState::new();
    let entry = InviteLinkEntry {
      creator_id: "user-1".to_string(),
      creator_username: "alice".to_string(),
      invite_type: message::signal::InviteType::Chat,
      room_id: None,
      expires_at: 9999999999999,
      used: false,
    };
    state
      .inner()
      .invite_links
      .insert("code-1".to_string(), entry);
    assert!(state.inner().invite_links.contains_key("code-1"));

    let link = state.inner().invite_links.get("code-1").unwrap();
    assert_eq!(link.creator_id, "user-1");
    assert_eq!(link.invite_type, message::signal::InviteType::Chat);
    assert!(!link.used);
  }

  #[test]
  fn test_active_invites_tracking() {
    let state = AppState::new();
    let key = "user-1->user-2:Chat".to_string();
    let expires_at = message::types::now_timestamp() + 30_000;

    state.inner().active_invites.insert(key.clone(), expires_at);
    assert!(state.inner().active_invites.contains_key(&key));

    // No longer exists after removal
    state.inner().active_invites.remove(&key);
    assert!(!state.inner().active_invites.contains_key(&key));
  }
}
