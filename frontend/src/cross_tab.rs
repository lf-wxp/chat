//! Cross-tab synchronization via BroadcastChannel.
//!
//! Enables multiple browser tabs of the same application to stay in
//! sync without duplicating WebSocket connections or WebRTC sessions.
//! Only one tab (the "leader") maintains the signaling connection;
//! other tabs receive state updates via BroadcastChannel messages.
//!
//! ## Synchronised state
//!
//! * Auth state changes (login/logout)
//! * Active conversation selection
//! * Message read receipts
//! * Settings changes (theme, language, notifications)
//! * Conversation list mutations (pin/mute/archive)

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::BroadcastChannel;

/// Channel name used for cross-tab communication.
const CHANNEL_NAME: &str = "chat-app-sync";

/// Message types for cross-tab synchronization.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum SyncMessage {
  /// Auth state changed (login/logout).
  AuthChanged { logged_in: bool },
  /// Active conversation changed.
  ConversationChanged { conv_id: Option<String> },
  /// A conversation was read (unread count cleared).
  ConversationRead { conv_id: String },
  /// Settings changed (key-value pair).
  SettingChanged { key: String, value: String },
  /// Conversation pinned/unpinned.
  PinChanged { conv_id: String, pinned: bool },
  /// Conversation muted/unmuted.
  MuteChanged { conv_id: String, muted: bool },
  /// Conversation archived/unarchived.
  ArchiveChanged { conv_id: String, archived: bool },
  /// Leader election heartbeat.
  Heartbeat { tab_id: String, timestamp_ms: i64 },
}

/// Cross-tab synchronization controller.
///
/// Wraps the `BroadcastChannel` API and provides typed message
/// send/receive with automatic JSON serialization.
pub struct CrossTabSync {
  channel: BroadcastChannel,
  _on_message: Closure<dyn FnMut(web_sys::MessageEvent)>,
  tab_id: String,
}

impl CrossTabSync {
  /// Create a new cross-tab sync instance.
  ///
  /// `on_message` is called whenever another tab broadcasts a sync
  /// message. The callback runs on the current tab's event loop.
  ///
  /// Returns `None` if the `BroadcastChannel` API is not available
  /// (e.g. in a non-browser environment or very old browsers).
  pub fn new<F>(on_message: F) -> Option<Self>
  where
    F: Fn(SyncMessage) + 'static,
  {
    let channel = BroadcastChannel::new(CHANNEL_NAME).ok()?;
    let tab_id = uuid::Uuid::new_v4().to_string();

    let closure = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
      let data = event.data();
      if let Some(json_str) = data.as_string()
        && let Ok(msg) = serde_json::from_str::<SyncMessage>(&json_str)
      {
        on_message(msg);
      }
    }) as Box<dyn FnMut(web_sys::MessageEvent)>);

    channel.set_onmessage(Some(closure.as_ref().unchecked_ref()));

    Some(Self {
      channel,
      _on_message: closure,
      tab_id,
    })
  }

  /// Broadcast a sync message to all other tabs.
  pub fn broadcast(&self, msg: &SyncMessage) {
    if let Ok(json) = serde_json::to_string(msg) {
      let _ = self.channel.post_message(&JsValue::from_str(&json));
    }
  }

  /// Get this tab's unique identifier.
  #[must_use]
  pub fn tab_id(&self) -> &str {
    &self.tab_id
  }
}

impl Drop for CrossTabSync {
  fn drop(&mut self) {
    self.channel.close();
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_sync_message_serialization() {
    let msg = SyncMessage::AuthChanged { logged_in: true };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("AuthChanged"));
    let decoded: SyncMessage = serde_json::from_str(&json).unwrap();
    assert!(matches!(
      decoded,
      SyncMessage::AuthChanged { logged_in: true }
    ));
  }

  #[test]
  fn test_sync_message_conversation_changed() {
    let msg = SyncMessage::ConversationChanged {
      conv_id: Some("test-conv".to_string()),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: SyncMessage = serde_json::from_str(&json).unwrap();
    assert!(matches!(
      decoded,
      SyncMessage::ConversationChanged { conv_id: Some(ref id) } if id == "test-conv"
    ));
  }

  #[test]
  fn test_sync_message_setting_changed() {
    let msg = SyncMessage::SettingChanged {
      key: "theme".to_string(),
      value: "dark".to_string(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: SyncMessage = serde_json::from_str(&json).unwrap();
    assert!(matches!(
      decoded,
      SyncMessage::SettingChanged { ref key, ref value }
        if key == "theme" && value == "dark"
    ));
  }
}
