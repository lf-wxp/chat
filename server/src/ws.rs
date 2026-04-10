//! WebSocket handling module.
//!
//! This module provides WebSocket connection management with:
//! - Binary message encoding/decoding using bitcode
//! - Heartbeat detection (Ping/Pong)
//! - Connection state management
//! - Graceful shutdown handling

use std::sync::Arc;
use std::time::Instant;

use axum::body::Bytes;
use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, State};
use axum::response::Response;
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use message::UserId;
use message::frame::{MessageFrame, decode_frame, encode_frame};
use message::signaling::{
  AuthFailure, AuthSuccess, Ping, Pong, SessionInvalidated, SignalingMessage, UserListUpdate,
  UserStatusChange,
};
use message::types::UserStatus;
use tokio::select;
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use crate::auth::UserStore;
use crate::config::Config;
use crate::logging::{desensitize_jwt, mask_ip};

/// Maximum pending messages in the send queue.
const SEND_QUEUE_SIZE: usize = 256;

/// WebSocket connection state shared across handlers.
#[derive(Debug, Clone)]
pub struct ConnectionState {
  /// User ID (set after authentication).
  pub user_id: Option<UserId>,
  /// Connection timestamp.
  pub connected_at: Instant,
  /// Last heartbeat received timestamp.
  pub last_heartbeat: Instant,
  /// Remote address.
  pub remote_addr: String,
}

impl ConnectionState {
  /// Create a new connection state.
  #[must_use]
  pub fn new(remote_addr: String) -> Self {
    Self {
      user_id: None,
      connected_at: Instant::now(),
      last_heartbeat: Instant::now(),
      remote_addr,
    }
  }
}

/// Global state for WebSocket connections.
#[derive(Debug)]
pub struct WebSocketState {
  /// Active connections: UserId -> Sender.
  connections: DashMap<UserId, mpsc::Sender<Vec<u8>>>,
  /// Connection metadata for logging.
  metadata: DashMap<UserId, ConnectionState>,
  /// User authentication store.
  user_store: UserStore,
  /// Configuration reference.
  config: Config,
}

impl WebSocketState {
  /// Create a new WebSocket state.
  #[must_use]
  pub fn new(config: Config, user_store: UserStore) -> Self {
    Self {
      connections: DashMap::new(),
      metadata: DashMap::new(),
      user_store,
      config,
    }
  }

  /// Add a connection for a user.
  pub fn add_connection(&self, user_id: UserId, sender: mpsc::Sender<Vec<u8>>) {
    self.connections.insert(user_id, sender);
  }

  /// Remove a connection for a user.
  pub fn remove_connection(&self, user_id: &UserId) {
    self.connections.remove(user_id);
    self.metadata.remove(user_id);
  }

  /// Get the sender for a user.
  pub fn get_sender(&self, user_id: &UserId) -> Option<mpsc::Sender<Vec<u8>>> {
    self
      .connections
      .get(user_id)
      .map(|entry| entry.value().clone())
  }

  /// Check if a user is connected.
  #[must_use]
  pub fn is_connected(&self, user_id: &UserId) -> bool {
    self.connections.contains_key(user_id)
  }

  /// Get all connected user IDs.
  #[must_use]
  pub fn connected_users(&self) -> Vec<UserId> {
    self
      .connections
      .iter()
      .map(|entry| entry.key().clone())
      .collect()
  }

  /// Get number of connected users.
  #[must_use]
  pub fn connection_count(&self) -> usize {
    self.connections.len()
  }

  /// Send a message to a specific user.
  pub async fn send_to(&self, user_id: &UserId, data: Vec<u8>) -> bool {
    if let Some(sender) = self.get_sender(user_id) {
      sender.send(data).await.is_ok()
    } else {
      false
    }
  }

  /// Broadcast a message to all connected users.
  pub async fn broadcast(&self, data: Vec<u8>) {
    for entry in self.connections.iter() {
      let sender = entry.value();
      if sender.send(data.clone()).await.is_err() {
        warn!("Failed to send broadcast message to user");
      }
    }
  }
}

/// WebSocket upgrade handler.
pub async fn ws_handler(
  ws: WebSocketUpgrade,
  State(ws_state): State<Arc<WebSocketState>>,
  ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
) -> Response {
  let remote_addr = addr.to_string();
  info!(
    remote_addr = %mask_ip(&remote_addr),
    "WebSocket connection request"
  );

  ws.on_upgrade(move |socket| handle_socket(socket, ws_state, remote_addr))
}

/// Handle WebSocket connection.
async fn handle_socket(socket: WebSocket, ws_state: Arc<WebSocketState>, remote_addr: String) {
  let mut conn_state = ConnectionState::new(remote_addr.clone());

  info!(
    remote_addr = %mask_ip(&remote_addr),
    "WebSocket connection established"
  );

  // Create message channels
  let (_tx, mut rx) = mpsc::channel::<Vec<u8>>(SEND_QUEUE_SIZE);
  let (mut socket_tx, mut socket_rx) = socket.split();

  // Heartbeat interval
  let mut heartbeat_interval = interval(ws_state.config.heartbeat_interval);
  let heartbeat_timeout = ws_state.config.heartbeat_timeout;

  // Main connection loop
  loop {
    select! {
      // Receive messages from client
      result = socket_rx.next() => {
        match result {
          Some(Ok(msg)) => {
            if !handle_incoming_message(&mut socket_tx, &ws_state, &mut conn_state, msg).await {
              break;
            }
          }
          Some(Err(e)) => {
            error!(
              remote_addr = %mask_ip(&remote_addr),
              error = %e,
              "WebSocket receive error"
            );
            break;
          }
          None => {
            info!(
              remote_addr = %mask_ip(&remote_addr),
              "WebSocket stream ended"
            );
            break;
          }
        }
      }

      // Send outgoing messages
      Some(data) = rx.recv() => {
        if socket_tx.send(Message::Binary(Bytes::from(data))).await.is_err() {
          warn!(
            remote_addr = %mask_ip(&remote_addr),
            "Failed to send message to client"
          );
          break;
        }
      }

      // Heartbeat check
      _ = heartbeat_interval.tick() => {
        // Check heartbeat timeout
        if conn_state.last_heartbeat.elapsed() > heartbeat_timeout {
          warn!(
            remote_addr = %mask_ip(&remote_addr),
            user_id = ?conn_state.user_id,
            elapsed_secs = conn_state.last_heartbeat.elapsed().as_secs(),
            "Heartbeat timeout, closing connection"
          );
          let _ = socket_tx.send(Message::Close(Some(CloseFrame {
            code: 1001,
            reason: "Heartbeat timeout".into(),
          }))).await;
          break;
        }

        // Send ping
        let ping_msg = SignalingMessage::Ping(Ping::default());
        if let Ok(encoded) = encode_signaling_message(&ping_msg) {
          if socket_tx.send(Message::Binary(Bytes::from(encoded))).await.is_err() {
            warn!("Failed to send ping");
            break;
          }
          debug!(
            remote_addr = %mask_ip(&remote_addr),
            "Sent ping"
          );
        }
      }
    }
  }

  // Cleanup on disconnect
  if let Some(user_id) = conn_state.user_id {
    ws_state.remove_connection(&user_id);
    info!(
      user_id = %user_id,
      remote_addr = %mask_ip(&remote_addr),
      connection_duration_secs = conn_state.connected_at.elapsed().as_secs(),
      "User disconnected"
    );
  } else {
    info!(
      remote_addr = %mask_ip(&remote_addr),
      connection_duration_secs = conn_state.connected_at.elapsed().as_secs(),
      "Unauthenticated connection closed"
    );
  }
}

/// Handle incoming WebSocket message.
/// Returns false if the connection should be closed.
async fn handle_incoming_message(
  socket_tx: &mut futures::stream::SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  conn_state: &mut ConnectionState,
  msg: Message,
) -> bool {
  match msg {
    Message::Binary(data) => {
      handle_binary_message(socket_tx, ws_state, conn_state, data.to_vec()).await
    }
    Message::Ping(data) => {
      // Respond with pong
      if socket_tx.send(Message::Pong(data)).await.is_err() {
        warn!("Failed to send pong");
        return false;
      }
      true
    }
    Message::Pong(_) => {
      // Update last heartbeat time
      conn_state.last_heartbeat = Instant::now();
      debug!(
        user_id = ?conn_state.user_id,
        "Received pong, heartbeat updated"
      );
      true
    }
    Message::Close(frame) => {
      info!(
        user_id = ?conn_state.user_id,
        close_frame = ?frame,
        "Client initiated close"
      );
      let _ = socket_tx.send(Message::Close(frame)).await;
      false
    }
    Message::Text(text) => {
      warn!(
        user_id = ?conn_state.user_id,
        text_len = text.len(),
        "Unexpected text message received, closing connection"
      );
      false
    }
  }
}

/// Handle binary message using message crate protocol.
async fn handle_binary_message(
  socket_tx: &mut futures::stream::SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  conn_state: &mut ConnectionState,
  data: Vec<u8>,
) -> bool {
  // Decode frame
  let frame = match decode_frame(&data) {
    Ok(frame) => frame,
    Err(e) => {
      warn!(
        user_id = ?conn_state.user_id,
        error = %e,
        "Failed to decode frame"
      );
      return true; // Continue connection
    }
  };

  // Decode signaling message
  let signaling_msg = match decode_signaling_message(&frame) {
    Ok(msg) => msg,
    Err(e) => {
      warn!(
        user_id = ?conn_state.user_id,
        error = %e,
        message_type = frame.message_type,
        "Failed to decode signaling message"
      );
      return true; // Continue connection
    }
  };

  // Handle message based on type
  handle_signaling_message(socket_tx, ws_state, conn_state, signaling_msg).await
}

/// Handle decoded signaling message.
async fn handle_signaling_message(
  socket_tx: &mut futures::stream::SplitSink<WebSocket, Message>,
  ws_state: &Arc<WebSocketState>,
  conn_state: &mut ConnectionState,
  msg: SignalingMessage,
) -> bool {
  match msg {
    SignalingMessage::Ping(_) => {
      // Respond with pong
      conn_state.last_heartbeat = Instant::now();
      let pong_msg = SignalingMessage::Pong(Pong::default());
      if let Ok(encoded) = encode_signaling_message(&pong_msg)
        && socket_tx
          .send(Message::Binary(Bytes::from(encoded)))
          .await
          .is_err()
      {
        warn!("Failed to send pong response");
        return false;
      }
      debug!(
        user_id = ?conn_state.user_id,
        "Responded to ping with pong"
      );
    }
    SignalingMessage::Pong(_) => {
      // Update heartbeat
      conn_state.last_heartbeat = Instant::now();
      debug!(
        user_id = ?conn_state.user_id,
        "Received pong"
      );
    }
    SignalingMessage::TokenAuth(auth) => {
      debug!(
        remote_addr = %mask_ip(&conn_state.remote_addr),
        token = %desensitize_jwt(&auth.token),
        "Received authentication request"
      );

      // Check if already authenticated
      if conn_state.user_id.is_some() {
        warn!(
          remote_addr = %mask_ip(&conn_state.remote_addr),
          "Connection already authenticated, rejecting re-auth"
        );
        let error_msg = SignalingMessage::AuthFailure(AuthFailure {
          reason: "Already authenticated".to_string(),
        });
        if let Ok(encoded) = encode_signaling_message(&error_msg) {
          let _ = socket_tx.send(Message::Binary(Bytes::from(encoded))).await;
        }
        return false;
      }

      // Authenticate with token
      match ws_state.user_store.authenticate_with_token(&auth.token) {
        Ok(auth_success) => {
          let user_id = auth_success.user_id.clone();

          // Check if this user is already connected (another session)
          if let Some(existing_sender) = ws_state.get_sender(&user_id) {
            // Send SessionInvalidated to the old connection
            let invalidated_msg = SignalingMessage::SessionInvalidated(SessionInvalidated::default());
            if let Ok(encoded) = encode_signaling_message(&invalidated_msg) {
              let _ = existing_sender.send(encoded).await;
            }
            // Remove old connection
            ws_state.remove_connection(&user_id);
            info!(
              user_id = %user_id,
              "Kicked existing session for single-device login"
            );
          }

          // Update connection state
          conn_state.user_id = Some(user_id.clone());
          conn_state.last_heartbeat = Instant::now();

          // Create send channel for this connection
          let (tx, _rx) = mpsc::channel(SEND_QUEUE_SIZE);
          ws_state.add_connection(user_id.clone(), tx);

          // Update user status to online
          ws_state.user_store.update_status(&user_id, UserStatus::Online);

          // Store metadata
          ws_state.metadata.insert(user_id.clone(), conn_state.clone());

          // Send AuthSuccess response
          let success_msg = SignalingMessage::AuthSuccess(AuthSuccess {
            user_id: user_id.clone(),
            username: auth_success.username,
          });
          if let Ok(encoded) = encode_signaling_message(&success_msg)
            && socket_tx.send(Message::Binary(Bytes::from(encoded))).await.is_err()
          {
            warn!("Failed to send auth success response");
            return false;
          }

          // Broadcast UserStatusChange to all other users
          let status_change = UserStatusChange {
            user_id: user_id.clone(),
            status: UserStatus::Online,
            signature: None,
          };
          if let Ok(encoded) = encode_signaling_message(&SignalingMessage::UserStatusChange(
            status_change,
          )) {
            // Broadcast to all connected users except self
            for entry in ws_state.connections.iter() {
              let other_user_id = entry.key();
              if other_user_id != &user_id {
                let sender = entry.value();
                let _ = sender.send(encoded.clone()).await;
              }
            }
          }

          // Send current online user list to the newly authenticated user
          let online_users = ws_state.user_store.get_online_users();
          let user_list_msg = SignalingMessage::UserListUpdate(UserListUpdate {
            users: online_users,
          });
          if let Ok(encoded) = encode_signaling_message(&user_list_msg) {
            let _ = socket_tx.send(Message::Binary(Bytes::from(encoded))).await;
          }

          info!(
            user_id = %user_id,
            remote_addr = %mask_ip(&conn_state.remote_addr),
            "User authenticated successfully"
          );
        }
        Err(auth_failure) => {
          warn!(
            remote_addr = %mask_ip(&conn_state.remote_addr),
            reason = %auth_failure.reason,
            "Authentication failed"
          );
          let failure_msg = SignalingMessage::AuthFailure(auth_failure);
          if let Ok(encoded) = encode_signaling_message(&failure_msg) {
            let _ = socket_tx.send(Message::Binary(Bytes::from(encoded))).await;
          }
          // Don't close connection, allow retry
        }
      }
    }
    SignalingMessage::UserLogout(_) => {
      if let Some(ref user_id) = conn_state.user_id {
        info!(
          user_id = %user_id,
          "User logout requested"
        );

        // Logout from user store
        ws_state.user_store.logout(user_id);

        // Broadcast UserStatusChange (Offline) to all other users
        let status_change = UserStatusChange {
          user_id: user_id.clone(),
          status: UserStatus::Offline,
          signature: None,
        };
        if let Ok(encoded) = encode_signaling_message(&SignalingMessage::UserStatusChange(
          status_change,
        )) {
          for entry in ws_state.connections.iter() {
            let other_user_id = entry.key();
            if other_user_id != user_id {
              let sender = entry.value();
              let _ = sender.send(encoded.clone()).await;
            }
          }
        }

        // Remove connection
        ws_state.remove_connection(user_id);
      }
      return false; // Close connection after logout
    }
    SignalingMessage::SessionInvalidated(_) => {
      // Client received this from server (should not happen)
      warn!(
        user_id = ?conn_state.user_id,
        "Received unexpected SessionInvalidated from client"
      );
    }
    _ => {
      // Check if user is authenticated for other message types
      if conn_state.user_id.is_none() {
        warn!(
          remote_addr = %mask_ip(&conn_state.remote_addr),
          message_type = ?std::mem::discriminant(&msg),
          "Unauthenticated user sent message, rejecting"
        );
        let error_msg = SignalingMessage::AuthFailure(AuthFailure {
          reason: "Authentication required".to_string(),
        });
        if let Ok(encoded) = encode_signaling_message(&error_msg) {
          let _ = socket_tx.send(Message::Binary(Bytes::from(encoded))).await;
        }
        return false;
      }

      // Route other messages to appropriate handlers
      debug!(
        user_id = ?conn_state.user_id,
        message_type = ?std::mem::discriminant(&msg),
        "Received signaling message"
      );
      // TODO: Implement message routing in later tasks
    }
  }

  true
}

/// Encode a signaling message to binary frame.
fn encode_signaling_message(
  msg: &SignalingMessage,
) -> Result<Vec<u8>, message::error::MessageError> {
  let discriminator = msg.discriminator();
  let payload = bitcode::encode(msg);
  let frame = MessageFrame::new(discriminator, payload);
  encode_frame(&frame)
}

/// Decode a signaling message from binary frame.
fn decode_signaling_message(
  frame: &MessageFrame,
) -> Result<SignalingMessage, message::error::MessageError> {
  bitcode::decode(&frame.payload).map_err(|e| {
    message::error::MessageError::Deserialization(format!(
      "Failed to decode signaling message: {e}"
    ))
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::auth::UserStore;
  use message::signaling::TokenAuth;

  fn create_test_config() -> Config {
    Config::default()
  }

  fn create_test_ws_state() -> WebSocketState {
    let config = create_test_config();
    let user_store = UserStore::new(&config);
    WebSocketState::new(config, user_store)
  }

  #[test]
  fn test_connection_state_new() {
    let state = ConnectionState::new("127.0.0.1:12345".to_string());
    assert!(state.user_id.is_none());
    assert_eq!(state.remote_addr, "127.0.0.1:12345");
  }

  #[test]
  fn test_websocket_state_new() {
    let state = create_test_ws_state();
    assert_eq!(state.connection_count(), 0);
  }

  #[test]
  fn test_websocket_state_add_remove() {
    let state = create_test_ws_state();
    let user_id = UserId::new();
    let (tx, _rx) = mpsc::channel(16);

    state.add_connection(user_id.clone(), tx);
    assert_eq!(state.connection_count(), 1);
    assert!(state.is_connected(&user_id));

    state.remove_connection(&user_id);
    assert_eq!(state.connection_count(), 0);
    assert!(!state.is_connected(&user_id));
  }

  #[test]
  fn test_encode_signaling_message() {
    let msg = SignalingMessage::TokenAuth(TokenAuth {
      token: "test_token".to_string(),
    });
    let result = encode_signaling_message(&msg);
    assert!(result.is_ok());

    let encoded = result.unwrap();
    // Should start with magic number 0xBCBC
    assert_eq!(encoded[0], 0xBC);
    assert_eq!(encoded[1], 0xBC);
    // Next byte is discriminator
    assert_eq!(encoded[2], 0x00); // TOKEN_AUTH discriminator
  }

  #[test]
  fn test_decode_signaling_message() {
    let msg = SignalingMessage::Ping(Ping::default());
    let encoded = encode_signaling_message(&msg).unwrap();
    let frame = decode_frame(&encoded).unwrap();
    let decoded = decode_signaling_message(&frame);

    assert!(decoded.is_ok());
    assert!(matches!(decoded.unwrap(), SignalingMessage::Ping(_)));
  }

  #[test]
  fn test_session_invalidated_message() {
    let msg = SignalingMessage::SessionInvalidated(SessionInvalidated::default());
    let result = encode_signaling_message(&msg);
    assert!(result.is_ok());

    let encoded = result.unwrap();
    assert_eq!(encoded[0], 0xBC);
    assert_eq!(encoded[1], 0xBC);
    assert_eq!(encoded[2], 0x07); // SESSION_INVALIDATED discriminator

    let frame = decode_frame(&encoded).unwrap();
    let decoded = decode_signaling_message(&frame);
    assert!(decoded.is_ok());
    assert!(matches!(
      decoded.unwrap(),
      SignalingMessage::SessionInvalidated(_)
    ));
  }
}
