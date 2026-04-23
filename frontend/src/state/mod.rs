//! Global application state.
//!
//! Centralized reactive state management using Leptos RwSignals.
//! All state is provided via context and accessed throughout the app.

use crate::utils;
use crate::webrtc::WebRtcState;
use leptos::prelude::*;
use message::RoomId;
use message::{
  UserId,
  types::{MemberInfo, NetworkQuality, RoomInfo, UserInfo, UserStatus},
};
use std::collections::HashMap;

/// Recovery phase for the reconnect banner (P2-1 fix, Req 10.11.40).
///
/// Distinguishes between a simple WebSocket reconnection and a full
/// page-refresh recovery where connections must be restored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryPhase {
  /// WebSocket is reconnecting (network interruption).
  Reconnecting,
  /// Auth recovery succeeded; restoring room/peer connections.
  RestoringConnections,
}

/// Conversation identifier.
///
/// Distinguishes between direct (1:1) and room-based conversations
/// using strongly typed identifiers rather than a shared type alias.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ConversationId {
  /// Direct message conversation identified by the peer's user ID.
  Direct(UserId),
  /// Group room conversation identified by the room ID.
  Room(RoomId),
}

/// Maximum number of pinned conversations.
pub const MAX_PINS: usize = 5;

/// Conversation model for sidebar and chat views.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Conversation {
  /// Unique conversation identifier
  pub id: ConversationId,
  /// Display name (user nickname or room name)
  pub display_name: String,
  /// Last message preview text
  pub last_message: Option<String>,
  /// Last message timestamp (unix ms)
  pub last_message_ts: Option<i64>,
  /// Unread message count
  pub unread_count: u32,
  /// Whether this conversation is pinned
  pub pinned: bool,
  /// Pin timestamp (for sorting)
  pub pinned_ts: Option<i64>,
  /// Whether this conversation is muted (do not disturb)
  pub muted: bool,
  /// Whether this conversation is archived
  pub archived: bool,
  /// Conversation type
  pub conversation_type: ConversationType,
}

/// Type of conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ConversationType {
  /// Direct message (1:1 chat)
  Direct,
  /// Group room chat
  Room,
}

/// Authentication state.
#[derive(Debug, Clone)]
pub struct AuthState {
  /// User ID
  pub user_id: UserId,
  /// JWT token
  pub token: String,
  /// Username (login name)
  pub username: String,
  /// Nickname (display name)
  pub nickname: String,
  /// Avatar data URI (Identicon or custom upload)
  pub avatar: String,
  /// Custom signature / status message (Req 10.1.6, Issue-5 fix).
  pub signature: String,
}

/// Global application state.
#[derive(Debug, Clone, Copy)]
pub struct AppState {
  /// Authentication state
  pub auth: RwSignal<Option<AuthState>>,
  /// Online users list
  pub online_users: RwSignal<Vec<UserInfo>>,
  /// Room list
  pub rooms: RwSignal<Vec<RoomInfo>>,
  /// Conversation list (with pinned/muted/archived state)
  pub conversations: RwSignal<Vec<Conversation>>,
  /// Currently active conversation
  pub active_conversation: RwSignal<Option<ConversationId>>,
  /// WebSocket connection state
  pub connected: RwSignal<bool>,
  /// Reconnecting state (for banner display)
  pub reconnecting: RwSignal<bool>,
  /// Recovery phase — distinguishes "Reconnecting..." from "Restoring
  /// connections..." in the banner (P2-1 fix, Req 10.11.40).
  pub recovery_phase: RwSignal<RecoveryPhase>,
  /// Network quality per peer
  pub network_quality: RwSignal<HashMap<UserId, NetworkQuality>>,
  /// Room members map: room_id → member list
  pub room_members: RwSignal<HashMap<RoomId, Vec<MemberInfo>>>,
  /// Current user's status (Online/Busy/Away/Offline)
  pub my_status: RwSignal<UserStatus>,
  /// Theme preference ("light" | "dark" | "system")
  pub theme: RwSignal<String>,
  /// Locale preference
  pub locale: RwSignal<String>,
  /// Debug mode enabled
  pub debug: RwSignal<bool>,
  /// Whether the settings drawer is currently open.
  pub settings_open: RwSignal<bool>,
  /// WebRTC peer connection and encryption state.
  pub webrtc_state: RwSignal<WebRtcState>,
}

impl AppState {
  /// Create new application state.
  #[must_use]
  pub fn new() -> Self {
    let theme = utils::load_from_local_storage("theme").unwrap_or_else(|| "system".to_string());
    let locale = utils::load_from_local_storage("locale").unwrap_or_else(Self::detect_locale);
    // Debug mode is enabled if EITHER localStorage has `debug_mode=true`
    // OR the URL contains `?debug=true` (P2-3 fix). Previously the URL
    // check was only a fallback when localStorage was absent.
    let debug = utils::load_from_local_storage("debug_mode")
      .map(|v| v == "true")
      .unwrap_or(false)
      || Self::detect_debug_from_url();
    Self {
      auth: RwSignal::new(None),
      online_users: RwSignal::new(Vec::new()),
      rooms: RwSignal::new(Vec::new()),
      conversations: RwSignal::new(Vec::new()),
      active_conversation: RwSignal::new(None),
      connected: RwSignal::new(false),
      reconnecting: RwSignal::new(false),
      recovery_phase: RwSignal::new(RecoveryPhase::Reconnecting),
      network_quality: RwSignal::new(HashMap::new()),
      room_members: RwSignal::new(HashMap::new()),
      my_status: RwSignal::new(UserStatus::Online),
      theme: RwSignal::new(theme),
      locale: RwSignal::new(locale),
      debug: RwSignal::new(debug),
      settings_open: RwSignal::new(false),
      webrtc_state: RwSignal::new(WebRtcState::new()),
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

  /// Get pinned conversations (sorted by pinned_ts desc).
  #[must_use]
  pub fn pinned_conversations(&self) -> Vec<Conversation> {
    let mut pinned: Vec<Conversation> = self
      .conversations
      .get()
      .into_iter()
      .filter(|c| c.pinned)
      .collect();
    pinned.sort_by_key(|c| std::cmp::Reverse(c.pinned_ts));
    pinned
  }

  /// Get non-archived conversations (excluding pinned, sorted by last_message_ts desc).
  #[must_use]
  pub fn active_conversations(&self) -> Vec<Conversation> {
    let mut active: Vec<Conversation> = self
      .conversations
      .get()
      .into_iter()
      .filter(|c| !c.archived && !c.pinned)
      .collect();
    active.sort_by_key(|c| std::cmp::Reverse(c.last_message_ts));
    active
  }

  /// Get archived conversations.
  #[must_use]
  pub fn archived_conversations(&self) -> Vec<Conversation> {
    self
      .conversations
      .get()
      .into_iter()
      .filter(|c| c.archived)
      .collect()
  }

  /// Toggle pin on a conversation (max 5).
  ///
  /// If the conversation is currently unpinned and the pin limit has been
  /// reached, this method does nothing and returns `false`.
  /// Returns `true` when the toggle was applied successfully.
  pub fn toggle_pin(&self, conversation_id: &ConversationId) -> bool {
    let mut applied = false;
    self.conversations.update(|convs| {
      // Count current pins before taking a mutable reference to the target.
      let current_pin_count = convs.iter().filter(|c| c.pinned).count();

      if let Some(conv) = convs.iter_mut().find(|c| c.id == *conversation_id) {
        if conv.pinned {
          // Unpin -- always allowed
          conv.pinned = false;
          conv.pinned_ts = None;
          applied = true;
        } else {
          // Pin -- check limit first (current_pin_count was computed above)
          if current_pin_count < MAX_PINS {
            conv.pinned = true;
            conv.pinned_ts = Some(chrono::Utc::now().timestamp_millis());
            conv.archived = false;
            applied = true;
          }
        }
      }
    });
    if applied {
      self.persist_conversations();
    }
    applied
  }

  /// Toggle mute on a conversation.
  pub fn toggle_mute(&self, conversation_id: &ConversationId) {
    self.conversations.update(|convs| {
      if let Some(conv) = convs.iter_mut().find(|c| c.id == *conversation_id) {
        conv.muted = !conv.muted;
      }
    });
    self.persist_conversations();
  }

  /// Toggle archive on a conversation.
  pub fn toggle_archive(&self, conversation_id: &ConversationId) {
    self.conversations.update(|convs| {
      if let Some(conv) = convs.iter_mut().find(|c| c.id == *conversation_id) {
        conv.archived = !conv.archived;
        if conv.archived {
          conv.pinned = false;
          conv.pinned_ts = None;
        }
      }
    });
    self.persist_conversations();
  }

  /// Persist conversation state to localStorage.
  fn persist_conversations(&self) {
    if let Some(window) = web_sys::window()
      && let Ok(Some(storage)) = window.local_storage()
    {
      let convs = self.conversations.get();
      if let Ok(json) = serde_json::to_string(&convs) {
        let _ = storage.set_item("conversations", &json);
      }
    }
  }

  /// Load conversations from localStorage.
  pub fn load_conversations(&self) {
    if let Some(window) = web_sys::window()
      && let Ok(Some(storage)) = window.local_storage()
      && let Ok(Some(json)) = storage.get_item("conversations")
      && let Ok(convs) = serde_json::from_str::<Vec<Conversation>>(&json)
    {
      self.conversations.set(convs);
    }
  }

  /// Persist `active_conversation` to localStorage (Req 10.9.34).
  fn persist_active_conversation(id: Option<&ConversationId>) {
    match id {
      Some(conv_id) => match serde_json::to_string(conv_id) {
        Ok(json) => utils::save_to_local_storage("active_conversation_id", &json),
        Err(_) => utils::remove_from_local_storage("active_conversation_id"),
      },
      None => utils::remove_from_local_storage("active_conversation_id"),
    }
  }

  /// Load the previously active conversation id from localStorage.
  fn load_active_conversation() -> Option<ConversationId> {
    let raw = utils::load_from_local_storage("active_conversation_id")?;
    if raw.is_empty() {
      return None;
    }
    serde_json::from_str(&raw).ok()
  }

  /// Detect locale from browser settings.
  fn detect_locale() -> String {
    if let Some(window) = web_sys::window()
      && let Some(lang) = window.navigator().language()
    {
      // Convert "zh-CN" -> "zh-CN", "en-US" -> "en"
      if lang.starts_with("zh") {
        return "zh-CN".to_string();
      }
      if lang.starts_with("en") {
        return "en".to_string();
      }
      // Other languages default to English per requirements
      return "en".to_string();
    }
    "en".to_string()
  }

  /// Detect debug mode from URL query parameter.
  ///
  /// Checks for `?debug=true` in the current page URL.
  /// This is used as a fallback when `localStorage.debug_mode` is not set.
  fn detect_debug_from_url() -> bool {
    if let Some(window) = web_sys::window() {
      let location = window.location();
      if let Ok(search) = location.search() {
        return search.contains("debug=true");
      }
    }
    false
  }
}

impl Default for AppState {
  fn default() -> Self {
    Self::new()
  }
}

// ── Context helpers ──

/// Provide AppState to the Leptos component tree.
pub fn provide_app_state() -> AppState {
  let state = AppState::new();
  state.load_conversations();

  // Restore the previously active conversation (Req 10.9.34). The Effect
  // below will persist any subsequent changes automatically.
  if let Some(id) = AppState::load_active_conversation() {
    state.active_conversation.set(Some(id));
  }

  // Persist `active_conversation` whenever it changes.
  let active = state.active_conversation;
  Effect::new(move |_| {
    let current = active.get();
    AppState::persist_active_conversation(current.as_ref());
  });

  provide_context(state);
  state
}

/// Retrieve AppState from the Leptos context.
///
/// # Panics
/// Panics if AppState has not been provided.
#[must_use]
pub fn use_app_state() -> AppState {
  expect_context::<AppState>()
}

#[cfg(test)]
mod tests;
