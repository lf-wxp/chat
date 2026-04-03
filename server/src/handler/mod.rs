//! WebSocket request handlers
//!
//! Split into the following submodules:
//! - `auth_handlers` — Registration, login, token authentication
//! - `signal_router` — Signal message routing and dispatch
//! - `room_handlers` — Room management (create/join/leave/kick/mute/transfer/screening room)
//! - `invite_handlers` — Invite links and connection invites
//! - `stats_handlers` — Statistics and monitoring API endpoints

pub mod auth_handlers;
pub mod invite_handlers;
pub mod room_handlers;
pub mod signal_router;
pub mod stats_handlers;

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::{
  extract::{
    ConnectInfo, State,
    ws::{Message, WebSocket},
  },
  response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use message::signal::SignalMessage;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::state::AppState;

/// Heartbeat timeout in seconds. If no Pong is received within this
/// duration the connection is considered dead and will be dropped.
const HEARTBEAT_INTERVAL_SECS: u64 = 15;
const HEARTBEAT_TIMEOUT_SECS: u64 = 30;

/// WebSocket upgrade handler
pub async fn ws_handler(
  ws: axum::extract::ws::WebSocketUpgrade,
  ConnectInfo(addr): ConnectInfo<SocketAddr>,
  State(state): State<AppState>,
) -> impl IntoResponse {
  info!("WebSocket handshake request: {}", addr);
  ws.on_upgrade(move |socket| handle_connection(socket, addr, state))
}

/// Handle a single WebSocket connection
async fn handle_connection(socket: WebSocket, addr: SocketAddr, state: AppState) {
  info!("WebSocket connection established: {}", addr);

  let (mut ws_sink, mut ws_stream) = socket.split();
  let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

  // Step 1: Wait for authentication message
  let user_id =
    if let Some(id) = auth_handlers::authenticate(&mut ws_stream, &mut ws_sink, &state).await {
      id
    } else {
      warn!("Authentication failed, disconnecting: {}", addr);
      return;
    };

  // Send ICE configuration after successful authentication
  let ice_config = SignalMessage::IceConfig {
    ice_servers: state.inner().ice_servers.clone(),
  };
  if let Ok(bytes) = bitcode::serialize(&ice_config) {
    let _ = ws_sink.send(Message::Binary(bytes.into())).await;
  }

  // Register connection
  let username = state
    .inner()
    .sessions
    .get(&user_id)
    .map(|s| s.username.clone())
    .unwrap_or_default();
  state.register_connection(user_id.clone(), username, tx);

  // Heartbeat tracking: set to true when a Pong (or any message) arrives
  let pong_received = Arc::new(AtomicBool::new(true));

  // Send task: receive messages from mpsc channel and send to WebSocket
  let send_task = {
    let user_id = user_id.clone();
    tokio::spawn(async move {
      while let Some(msg) = rx.recv().await {
        if ws_sink.send(msg).await.is_err() {
          break;
        }
      }
      info!("Send task ended: {}", user_id);
    })
  };

  // Heartbeat timer: periodically check if Pong is received, disconnect on timeout
  let heartbeat_task = {
    let state = state.clone();
    let user_id = user_id.clone();
    let pong_received = Arc::clone(&pong_received);
    tokio::spawn(async move {
      let mut interval =
        tokio::time::interval(tokio::time::Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
      loop {
        interval.tick().await;
        // Check if we received a pong since last tick
        if pong_received.swap(false, Ordering::Relaxed) {
          // Send a Ping to the client
          state.send_to_user(&user_id, &SignalMessage::Ping);
        } else {
          // No pong received within the interval — timeout
          warn!(
            "Heartbeat timeout, disconnecting user {} (no response for {}s)",
            user_id, HEARTBEAT_TIMEOUT_SECS
          );
          break;
        }
      }
    })
  };

  // Receive task: receive messages from WebSocket and process
  let recv_task = {
    let state = state.clone();
    let user_id = user_id.clone();
    let pong_received = Arc::clone(&pong_received);
    tokio::spawn(async move {
      while let Some(Ok(msg)) = ws_stream.next().await {
        match msg {
          Message::Binary(data) => {
            // Any valid message counts as "alive"
            pong_received.store(true, Ordering::Relaxed);
            match bitcode::deserialize::<SignalMessage>(&data) {
              Ok(signal) => {
                signal_router::handle_signal(&user_id, signal, &state);
              }
              Err(e) => {
                warn!("Failed to deserialize signal message: {}", e);
              }
            }
          }
          Message::Close(_) => break,
          Message::Ping(_data) => {
            pong_received.store(true, Ordering::Relaxed);
          }
          _ => {}
        }
      }
      info!("Receive task ended: {}", user_id);
    })
  };

  // Wait for any task to complete
  tokio::select! {
    _ = send_task => {},
    _ = recv_task => {},
    _ = heartbeat_task => {},
  }

  info!("{} disconnected", addr);
  state.unregister_connection(&user_id);
}

/// Send signal message to WebSocket
pub async fn send_signal(
  sink: &mut futures::stream::SplitSink<WebSocket, Message>,
  msg: &SignalMessage,
) -> Result<(), Box<dyn std::error::Error>> {
  let bytes = bitcode::serialize(msg)?;
  sink.send(Message::Binary(bytes.into())).await?;
  Ok(())
}
