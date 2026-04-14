//! Common integration test helpers shared across all integration test files.
//!
//! Provides reusable utilities for:
//! - Test server creation and WebSocket connection
//! - Signaling message send/receive with configurable filtering
//! - User authentication helpers
//! - Message draining utilities

// Each integration test binary includes this module but may not use every helper.
#![allow(dead_code)]

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use futures::{SinkExt, StreamExt};
use message::frame::{decode_frame, encode_frame};
use message::signaling::{SignalingMessage, TokenAuth};
use server::auth::UserStore;
use server::config::Config;
use server::ws::WebSocketState;
use tokio::net::TcpListener;
use tokio::time::{sleep, timeout};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};

/// Convenient type alias for the WebSocket stream used in integration tests.
pub type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// Create a test server bound to a random port and return its address,
/// shared WebSocket state, and user store.
pub async fn create_test_server() -> (SocketAddr, Arc<WebSocketState>, UserStore) {
  let config = Config::default();
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
      app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
  });

  sleep(Duration::from_millis(100)).await;

  (addr, ws_state, user_store)
}

/// Connect to the WebSocket server at the given address.
pub async fn connect_ws(addr: SocketAddr) -> WsStream {
  let url = format!("ws://{}/ws", addr);
  let (ws_stream, _) = connect_async(&url).await.unwrap();
  ws_stream
}

/// Send a signaling message over the WebSocket connection.
pub async fn send_signaling(ws: &mut WsStream, msg: &SignalingMessage) {
  let discriminator = msg.discriminator();
  let payload = bitcode::encode(msg);
  let frame = message::frame::MessageFrame::new(discriminator, payload);
  let encoded = encode_frame(&frame).unwrap();
  ws.send(Message::Binary(encoded.into())).await.unwrap();
}

/// Receive a signaling message, skipping any message for which `should_skip` returns true.
///
/// This is the core receive helper — each test file can wrap it with its own skip predicate
/// to filter out different sets of "noise" messages depending on what the test needs.
///
/// Returns `None` if no matching message arrives within 5 seconds.
pub async fn recv_signaling_filtered<F>(
  ws: &mut WsStream,
  should_skip: F,
) -> Option<SignalingMessage>
where
  F: Fn(&SignalingMessage) -> bool,
{
  timeout(Duration::from_secs(5), async {
    loop {
      match ws.next().await? {
        Ok(Message::Binary(data)) => {
          let frame = decode_frame(&data).ok()?;
          let msg: SignalingMessage = bitcode::decode(&frame.payload).ok()?;
          if should_skip(&msg) {
            continue;
          }
          return Some(msg);
        }
        Ok(Message::Ping(_) | Message::Pong(_)) => continue,
        _ => return None,
      }
    }
  })
  .await
  .ok()?
}

/// Register a user, connect via WebSocket, authenticate, and skip the initial
/// AuthSuccess + UserListUpdate messages.
///
/// Returns the authenticated WebSocket stream and the user's ID.
pub async fn auth_user(
  addr: SocketAddr,
  user_store: &UserStore,
  username: &str,
  password: &str,
) -> (WsStream, message::UserId) {
  let (user_id, token) = user_store.register(username, password).unwrap();
  let mut ws = connect_ws(addr).await;

  send_signaling(&mut ws, &SignalingMessage::TokenAuth(TokenAuth { token })).await;

  // Wait for AuthSuccess (skip only heartbeat noise)
  let response = recv_signaling_filtered(&mut ws, |msg| {
    matches!(msg, SignalingMessage::Ping(_) | SignalingMessage::Pong(_))
  })
  .await;
  assert!(
    matches!(response, Some(SignalingMessage::AuthSuccess(_))),
    "Expected AuthSuccess after TokenAuth, got: {:?}",
    response
  );

  // Wait for UserListUpdate or ActivePeersList
  let _ = recv_signaling_filtered(&mut ws, |msg| {
    matches!(msg, SignalingMessage::Ping(_) | SignalingMessage::Pong(_))
  })
  .await;

  (ws, user_id)
}

/// Drain all pending messages from the WebSocket buffer.
///
/// Optionally waits `wait` duration before starting to drain, then reads and discards
/// every message that arrives within 50ms windows until no more messages come.
pub async fn drain_messages(ws: &mut WsStream, wait: Duration) {
  sleep(wait).await;
  loop {
    match timeout(Duration::from_millis(50), ws.next()).await {
      Ok(Some(Ok(Message::Binary(data)))) => {
        if let Ok(frame) = decode_frame(&data) {
          let _: Option<SignalingMessage> = bitcode::decode(&frame.payload).ok();
        }
      }
      Ok(Some(Ok(Message::Ping(_) | Message::Pong(_)))) => continue,
      _ => break,
    }
  }
}
