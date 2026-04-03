//! Test helper utilities module
//!
//! Provides common test utility functions for constructing AppState, registering mock connections, etc.

use axum::extract::ws::Message;
use message::signal::{SignalMessage, UserStatus};
use server::auth::{self, UserSession};
use server::state::{AppState, ConnectionHandle};
use tokio::sync::mpsc;

/// Create a clean AppState instance
pub fn new_state() -> AppState {
  AppState::new()
}

/// Register a mock user connection, returns the receiver for verifying sent messages
pub fn register_mock_user(
  state: &AppState,
  user_id: &str,
  username: &str,
) -> mpsc::UnboundedReceiver<Message> {
  let (tx, rx) = mpsc::unbounded_channel();

  // Create user session
  let session = UserSession {
    user_id: user_id.to_string(),
    username: username.to_string(),
    password_hash: auth::hash_password("test_password").expect("Hashing failed"),
    status: UserStatus::Online,
    avatar: None,
    signature: None,
  };
  state.inner().sessions.insert(user_id.to_string(), session);
  state
    .inner()
    .username_map
    .insert(username.to_string(), user_id.to_string());

  // Register connection (without using register_connection to avoid triggering broadcast)
  state.inner().connections.insert(
    user_id.to_string(),
    ConnectionHandle {
      user_id: user_id.to_string(),
      username: username.to_string(),
      tx,
    },
  );

  rx
}

/// Extract all sent SignalMessages from the receiver
pub fn drain_messages(rx: &mut mpsc::UnboundedReceiver<Message>) -> Vec<SignalMessage> {
  let mut messages = Vec::new();
  while let Ok(msg) = rx.try_recv() {
    if let Message::Binary(data) = msg {
      if let Ok(signal) = bitcode::deserialize::<SignalMessage>(&data) {
        messages.push(signal);
      }
    }
  }
  messages
}

/// Check if the receiver contains a message of the specified type (via closure matching)
pub fn has_message<F>(rx: &mut mpsc::UnboundedReceiver<Message>, predicate: F) -> bool
where
  F: Fn(&SignalMessage) -> bool,
{
  let messages = drain_messages(rx);
  messages.iter().any(predicate)
}
