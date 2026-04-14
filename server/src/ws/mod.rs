//! WebSocket handling module.
//!
//! This module provides WebSocket connection management with:
//! - Binary message encoding/decoding using bitcode
//! - Heartbeat detection (Ping/Pong)
//! - Connection state management
//! - Graceful shutdown handling

mod call;
mod handler;
mod invite;
mod room;
mod theater;
mod utils;
mod webrtc;

use std::sync::Arc;
use std::time::Instant;

use axum::body::Bytes;
use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, State};
use axum::response::Response;
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use message::UserId;
use message::signaling::{ModerationNotification, Ping, SignalingMessage};
use tokio::select;
use tokio::sync::mpsc;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use crate::auth::UserStore;
use crate::config::Config;
use crate::discovery::DiscoveryState;
use crate::logging::mask_ip;

/// Background cleanup task interval in seconds.
///
/// This interval controls how often the server performs periodic cleanup:
/// - Expired mute auto-unmute
/// - Expired invitation cleanup
/// - Expired multi-invite cleanup
/// - Expired SDP negotiation cleanup
const BACKGROUND_CLEANUP_INTERVAL_SECS: u64 = 30;
use crate::room::RoomState;

pub use utils::{decode_signaling_message, encode_signaling_message};

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
  /// Sender for outgoing messages (set after authentication).
  pub sender: Option<mpsc::Sender<Vec<u8>>>,
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
      sender: None,
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
  /// User discovery and invitation state.
  discovery_state: DiscoveryState,
  /// Room management state.
  room_state: RoomState,
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
      discovery_state: DiscoveryState::new(),
      room_state: RoomState::new(),
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

  /// Spawn background periodic cleanup tasks with cancellation support.
  ///
  /// Runs the following cleanup operations every 30 seconds:
  /// - Expired mute auto-unmute with notifications
  /// - Expired invitation cleanup
  /// - Expired multi-invite cleanup
  /// - Expired SDP negotiation cleanup
  ///
  /// The tasks will be gracefully cancelled when the provided
  /// `CancellationToken` is triggered during server shutdown.
  pub fn spawn_background_tasks(self: &Arc<Self>, cancel_token: CancellationToken) {
    let ws_state = Arc::clone(self);
    tokio::spawn(async move {
      let mut cleanup_interval = interval(std::time::Duration::from_secs(
        BACKGROUND_CLEANUP_INTERVAL_SECS,
      ));
      loop {
        select! {
          () = cancel_token.cancelled() => {
            info!("Background cleanup tasks cancelled, shutting down");
            break;
          }
          _ = cleanup_interval.tick() => {
            // 1. Check expired mutes and send notifications
            let expired_mutes = ws_state.room_state.check_expired_mutes();
            for (room_id, user_ids) in &expired_mutes {
              for user_id in user_ids {
                // Send ModerationNotification to the unmuted user
                let notification = SignalingMessage::ModerationNotification(ModerationNotification {
                  room_id: room_id.clone(),
                  action: message::signaling::ModerationAction::Unmuted,
                  target: user_id.clone(),
                  reason: Some("Mute duration expired".to_string()),
                  duration_secs: None,
                });
                if let Ok(encoded) = encode_signaling_message(&notification)
                  && let Some(sender) = ws_state.get_sender(user_id)
                {
                  let _ = sender.send(encoded).await;
                }

                // Broadcast MuteStatusChange to room members
                if let Some(room) = ws_state.room_state.get_room(room_id) {
                  let mute_status =
                    SignalingMessage::MuteStatusChange(message::signaling::MuteStatusChange {
                      room_id: room_id.clone(),
                      target: user_id.clone(),
                      mute_info: message::types::MuteInfo::NotMuted,
                    });
                  if let Ok(encoded) = encode_signaling_message(&mute_status) {
                    for m in room.get_members() {
                      if let Some(sender) = ws_state.get_sender(&m.user_id) {
                        let _ = sender.send(encoded.clone()).await;
                      }
                    }
                  }
                }

                info!(
                  room_id = %room_id,
                  user_id = %user_id,
                  "Mute expired automatically"
                );
              }
            }

            // 2. Clean up timed-out invitations
            let timed_out = ws_state.discovery_state.get_timed_out_invitations();
            if !timed_out.is_empty() {
              debug!(count = timed_out.len(), "Cleaned up timed-out invitations");
            }

            // 3. Clean up expired multi-invites
            ws_state.discovery_state.cleanup_expired_multi_invites();

            // 4. Clean up expired SDP negotiations
            ws_state.discovery_state.cleanup_expired_sdp_negotiations();

            // 5. Clean up old log files based on config limits
            if ws_state.config.log_output != "stdout"
              && let Err(e) = crate::logging::cleanup_old_logs(
                &ws_state.config.log_dir,
                ws_state.config.log_max_files,
                ws_state.config.log_max_size_mb,
              )
            {
              warn!(error = %e, "Failed to clean up old log files");
            }
          }
        }
      }
    });

    info!("Background cleanup tasks spawned (interval: 30s)");
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

  // Create message channels for outgoing messages
  // tx is stored in conn_state.sender after authentication
  let (tx, mut rx) = mpsc::channel::<Vec<u8>>(ws_state.config.send_queue_size);
  conn_state.sender = Some(tx);

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
            if !handler::handle_incoming_message(&mut socket_tx, &ws_state, &mut conn_state, msg).await {
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
    handler::handle_user_disconnect(&ws_state, &user_id).await;
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

#[cfg(test)]
mod tests;
