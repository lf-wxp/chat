//! WebSocket connection management

use axum::extract::ws::Message;
use message::signal::{OnlineUser, SignalMessage, UserStatus};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::state::AppState;

impl AppState {
  /// Send signal message to a specific user
  pub fn send_to_user(&self, user_id: &str, msg: &SignalMessage) {
    if let Some(conn) = self.inner().connections.get(user_id)
      && let Ok(bytes) = bitcode::serialize(msg)
      && conn.tx.send(Message::Binary(bytes.into())).is_err()
    {
      warn!("Failed to send message to user {}", user_id);
    }
  }

  /// Broadcast signal message to all online users (excluding specified user)
  pub fn broadcast(&self, msg: &SignalMessage, exclude: Option<&str>) {
    if let Ok(bytes) = bitcode::serialize(msg) {
      let binary_msg = Message::Binary(bytes.into());
      for entry in &self.inner().connections {
        if exclude.is_some_and(|id| id == entry.key()) {
          continue;
        }
        if entry.value().tx.send(binary_msg.clone()).is_err() {
          warn!("Failed to broadcast message to user {}", entry.key());
        }
      }
    }
  }

  /// Broadcast online user list update
  pub fn broadcast_user_list(&self) {
    let users: Vec<OnlineUser> = self
      .inner()
      .connections
      .iter()
      .map(|entry| {
        let session = self.inner().sessions.get(entry.key());
        let status = session.as_ref().map_or(UserStatus::Online, |s| s.status);
        let avatar = session.as_ref().and_then(|s| s.avatar.clone());
        OnlineUser {
          user_id: entry.key().clone(),
          username: entry.value().username.clone(),
          status,
          avatar,
        }
      })
      .collect();

    let msg = SignalMessage::UserListUpdate { users };
    self.broadcast(&msg, None);
  }

  /// Register a new WebSocket connection
  pub fn register_connection(
    &self,
    user_id: String,
    username: String,
    tx: mpsc::UnboundedSender<Message>,
  ) {
    self.inner().connections.insert(
      user_id.clone(),
      crate::state::ConnectionHandle {
        user_id: user_id.clone(),
        username,
        tx,
      },
    );
    info!("User {} connected", user_id);

    // Push offline invites
    if let Some((_, invites)) = self.inner().pending_invites.remove(&user_id) {
      for invite in invites {
        self.send_to_user(&user_id, &invite);
      }
    }

    // Broadcast user list update
    self.broadcast_user_list();
  }

  /// Unregister a WebSocket connection
  pub fn unregister_connection(&self, user_id: &str) {
    self.inner().connections.remove(user_id);
    info!("User {} disconnected", user_id);

    // Broadcast user list update
    self.broadcast_user_list();
  }
}
