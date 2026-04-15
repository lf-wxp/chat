//! Integration tests for WebSocket heartbeat and timeout behavior.
//!
//! Tests the heartbeat mechanism including:
//! - Heartbeat Ping/Pong exchange
//! - Heartbeat timeout detection
//! - Connection cleanup on timeout

mod common;

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use common::{WsStream, connect_ws, send_signaling};
use message::signaling::{Pong, SignalingMessage, TokenAuth};
use server::auth::UserStore;
use server::config::Config;
use server::ws::WebSocketState;
use tokio::net::TcpListener;
use tokio::time::{sleep, timeout};
use tokio_tungstenite::tungstenite::Message;

/// Create a test server with custom heartbeat configuration.
async fn create_test_server_with_heartbeat(
  heartbeat_interval_secs: u64,
  heartbeat_timeout_secs: u64,
) -> (std::net::SocketAddr, Arc<WebSocketState>, UserStore) {
  let config = Config {
    heartbeat_interval: Duration::from_secs(heartbeat_interval_secs),
    heartbeat_timeout: Duration::from_secs(heartbeat_timeout_secs),
    ..Default::default()
  };

  let user_store = UserStore::new(&config);
  let ws_state = Arc::new(WebSocketState::new(config.clone(), user_store.clone()));

  let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
  let addr = listener.local_addr().unwrap();

  let app = Router::new()
    .route("/ws", axum::routing::get(server::ws::ws_handler))
    .with_state(ws_state.clone());

  tokio::spawn(async move {
    axum::serve(
      listener,
      app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .unwrap();
  });

  sleep(Duration::from_millis(100)).await;

  (addr, ws_state, user_store)
}

/// Receive a signaling message, skipping non-heartbeat messages.
async fn recv_heartbeat(ws: &mut WsStream) -> Option<SignalingMessage> {
  timeout(Duration::from_secs(5), async {
    use futures::StreamExt;
    loop {
      match ws.next().await? {
        Ok(Message::Binary(data)) => {
          let frame = message::frame::decode_frame(&data).ok()?;
          let msg: SignalingMessage = bitcode::decode(&frame.payload).ok()?;
          if matches!(msg, SignalingMessage::Ping(_) | SignalingMessage::Pong(_)) {
            return Some(msg);
          }
          // Skip non-heartbeat messages
        }
        Ok(Message::Ping(_) | Message::Pong(_)) => continue,
        _ => return None,
      }
    }
  })
  .await
  .ok()?
}

/// Helper to register, connect, and authenticate a user.
async fn auth_user_quick(
  addr: std::net::SocketAddr,
  user_store: &UserStore,
  username: &str,
) -> (WsStream, message::UserId) {
  let (user_id, token) = user_store.register(username, "password").unwrap();
  let mut ws = connect_ws(addr).await;

  send_signaling(&mut ws, &SignalingMessage::TokenAuth(TokenAuth { token })).await;

  // Wait for AuthSuccess
  use futures::StreamExt;
  timeout(Duration::from_secs(2), async {
    loop {
      match ws.next().await? {
        Ok(Message::Binary(data)) => {
          let frame = message::frame::decode_frame(&data).ok()?;
          let msg: SignalingMessage = bitcode::decode(&frame.payload).ok()?;
          if matches!(msg, SignalingMessage::AuthSuccess(_)) {
            return Some(());
          }
        }
        _ => continue,
      }
    }
  })
  .await
  .ok()
  .flatten();

  // Skip initial messages
  sleep(Duration::from_millis(100)).await;

  (ws, user_id)
}

// =============================================================================
// Heartbeat Exchange Tests
// =============================================================================

#[tokio::test]
async fn test_heartbeat_ping_received() {
  // Use short interval to speed up test
  let (addr, _ws_state, user_store) = create_test_server_with_heartbeat(1, 2).await;
  let (mut ws, _user_id) = auth_user_quick(addr, &user_store, "test_user").await;

  // Should receive a Ping within heartbeat interval + margin
  let ping = recv_heartbeat(&mut ws).await;
  assert!(
    matches!(ping, Some(SignalingMessage::Ping(_))),
    "Expected Ping message, got: {:?}",
    ping
  );
}

#[tokio::test]
async fn test_heartbeat_pong_response() {
  let (addr, _ws_state, user_store) = create_test_server_with_heartbeat(1, 2).await;
  let (mut ws, _user_id) = auth_user_quick(addr, &user_store, "test_user").await;

  // Wait for Ping
  let ping = recv_heartbeat(&mut ws).await;
  if let Some(SignalingMessage::Ping(_)) = ping {
    // Send Pong response
    send_signaling(&mut ws, &SignalingMessage::Pong(Pong {})).await;

    // Connection should remain open
    sleep(Duration::from_millis(500)).await;

    // Should be able to receive another Ping
    let next_ping = recv_heartbeat(&mut ws).await;
    assert!(
      matches!(next_ping, Some(SignalingMessage::Ping(_))),
      "Expected another Ping after Pong, connection should still be alive"
    );
  } else {
    panic!("Expected Ping message first");
  }
}

// =============================================================================
// Heartbeat Timeout Tests
// =============================================================================

#[tokio::test]
async fn test_heartbeat_timeout_disconnects_client() {
  // Use very short interval and timeout for testing
  let (addr, ws_state, user_store) = create_test_server_with_heartbeat(1, 2).await;
  let (mut ws, user_id) = auth_user_quick(addr, &user_store, "test_user").await;

  // Verify user is connected
  assert!(
    ws_state.is_connected(&user_id),
    "User should be connected initially"
  );

  // Receive Ping but DO NOT respond with Pong
  let ping = recv_heartbeat(&mut ws).await;
  assert!(
    matches!(ping, Some(SignalingMessage::Ping(_))),
    "Expected Ping message"
  );

  // Wait for timeout + cleanup margin
  // The server should close the connection after heartbeat_timeout
  sleep(Duration::from_secs(3)).await;

  // Check that user is no longer connected
  // Note: The WebSocket connection state may take a moment to clean up
  let mut disconnected = false;
  for _ in 0..10 {
    if !ws_state.is_connected(&user_id) {
      disconnected = true;
      break;
    }
    sleep(Duration::from_millis(200)).await;
  }

  assert!(
    disconnected,
    "User should be disconnected after heartbeat timeout"
  );
}

#[tokio::test]
async fn test_multiple_clients_timeout_independence() {
  let (addr, ws_state, user_store) = create_test_server_with_heartbeat(1, 2).await;

  // Create two clients
  let (mut ws1, user1) = auth_user_quick(addr, &user_store, "user1").await;
  let (mut ws2, user2) = auth_user_quick(addr, &user_store, "user2").await;

  // Both users should be connected
  assert!(ws_state.is_connected(&user1), "User1 should be connected");
  assert!(ws_state.is_connected(&user2), "User2 should be connected");

  // User1 stops responding to heartbeats
  let _ = recv_heartbeat(&mut ws1).await;

  // User2 keeps responding to heartbeats throughout the test
  // We need to respond multiple times during the wait period

  // Spawn a task to keep responding to heartbeats for User2
  let handle = tokio::spawn(async move {
    for _ in 0..10 {
      if let Some(SignalingMessage::Ping(_)) = recv_heartbeat(&mut ws2).await {
        send_signaling(&mut ws2, &SignalingMessage::Pong(Pong {})).await;
      }
      sleep(Duration::from_millis(300)).await;
    }
  });

  // Wait for User1 timeout (heartbeat_timeout=2s, plus margin)
  sleep(Duration::from_secs(3)).await;

  // User1 should be disconnected
  let mut user1_disconnected = false;
  for _ in 0..10 {
    if !ws_state.is_connected(&user1) {
      user1_disconnected = true;
      break;
    }
    sleep(Duration::from_millis(200)).await;
  }
  assert!(
    user1_disconnected,
    "User1 should be disconnected after timeout"
  );

  // User2 should still be connected (they responded to heartbeats)
  assert!(
    ws_state.is_connected(&user2),
    "User2 should still be connected (responded to heartbeats)"
  );

  // Cleanup
  handle.abort();
}

#[tokio::test]
async fn test_connection_remains_open_with_pong_response() {
  let (addr, ws_state, user_store) = create_test_server_with_heartbeat(1, 3).await;
  let (mut ws, user_id) = auth_user_quick(addr, &user_store, "test_user").await;

  // Keep responding to heartbeats for multiple cycles
  for _ in 0..5 {
    if let Some(SignalingMessage::Ping(_)) = recv_heartbeat(&mut ws).await {
      send_signaling(&mut ws, &SignalingMessage::Pong(Pong {})).await;
    }
    sleep(Duration::from_millis(500)).await;
  }

  // User should still be connected after multiple heartbeat cycles
  assert!(
    ws_state.is_connected(&user_id),
    "User should remain connected when responding to heartbeats"
  );
}
