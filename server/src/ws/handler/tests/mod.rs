//! Tests for WebSocket message handler.
//!
//! This module tests the handler.rs functions:
//! - handle_incoming_message (public)
//! - handle_user_disconnect (public)
//! - handle_binary_message (internal)
//! - handle_signaling_message (internal)

mod binary_message;
mod disconnect;
mod incoming_message;
mod token_auth;

// =============================================================================
// Mock Sink for Testing
// =============================================================================

/// A simple mock sink that stores sent messages for verification.
/// Uses UnboundedSender for simplicity in testing.
pub(crate) struct MockSink {
  tx: mpsc::UnboundedSender<Message>,
}

impl MockSink {
  /// Create a new MockSink with a receiver to read sent messages.
  pub(crate) fn new() -> (Self, mpsc::UnboundedReceiver<Message>) {
    let (tx, rx) = mpsc::unbounded_channel();
    (Self { tx }, rx)
  }
}

impl Sink<Message> for MockSink {
  type Error = std::io::Error;

  fn poll_ready(
    self: std::pin::Pin<&mut Self>,
    _cx: &mut Context<'_>,
  ) -> Poll<Result<(), Self::Error>> {
    // UnboundedSender::send is always ready
    Poll::Ready(Ok(()))
  }

  fn start_send(self: std::pin::Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
    self.tx.send(item).map_err(std::io::Error::other)
  }

  fn poll_flush(
    self: std::pin::Pin<&mut Self>,
    _cx: &mut Context<'_>,
  ) -> Poll<Result<(), Self::Error>> {
    Poll::Ready(Ok(()))
  }

  fn poll_close(
    self: std::pin::Pin<&mut Self>,
    _cx: &mut Context<'_>,
  ) -> Poll<Result<(), Self::Error>> {
    Poll::Ready(Ok(()))
  }
}

// =============================================================================
// Test Context
// =============================================================================

/// Test context for handler tests.
/// Provides WebSocketState, ConnectionState, and mock channels.
#[allow(dead_code)]
pub(crate) struct TestContext {
  /// WebSocket state (shared)
  pub ws_state: Arc<WebSocketState>,
  /// Connection state (mutable)
  pub conn_state: ConnectionState,
  /// Mock sink for sending messages to client
  pub mock_sink: MockSink,
  /// Receiver for reading messages sent to client
  pub rx: mpsc::UnboundedReceiver<Message>,
  /// Channel for outgoing messages (stored in conn_state.sender)
  pub tx: mpsc::Sender<Vec<u8>>,
  /// Receiver for outgoing messages
  pub outgoing_rx: mpsc::Receiver<Vec<u8>>,
}

impl TestContext {
  /// Create a new test context with unauthenticated user.
  pub(crate) fn new() -> Self {
    let config = Config::default();
    let user_store = UserStore::new(&config);
    let ws_state = Arc::new(WebSocketState::new(config, user_store));

    let conn_state = ConnectionState::new("127.0.0.1:12345".to_string());

    let (mock_sink, rx) = MockSink::new();
    let (tx, outgoing_rx) = mpsc::channel(100);

    Self {
      ws_state,
      conn_state,
      mock_sink,
      rx,
      tx,
      outgoing_rx,
    }
  }

  /// Create a test context with an authenticated user.
  pub(crate) fn with_authenticated_user(user_id_str: &str) -> Self {
    let mut ctx = Self::new();

    // Register and login the user to get a valid token
    let (user_id, _token) = ctx
      .ws_state
      .user_store
      .register(user_id_str, "test_password")
      .expect("Failed to register user");

    // Authenticate the user
    ctx.conn_state.user_id = Some(user_id.clone());
    ctx.conn_state.sender = Some(ctx.tx.clone());
    ctx.ws_state.add_connection(user_id.clone(), ctx.tx.clone());

    // Update user status to Online
    ctx
      .ws_state
      .user_store
      .update_status(&user_id, UserStatus::Online);

    // Store metadata
    ctx
      .ws_state
      .metadata
      .insert(user_id, ctx.conn_state.clone());

    ctx
  }

  /// Get the mock sink as a Sink + Send type for testing.
  #[allow(dead_code)]
  pub(crate) fn get_sink(&mut self) -> &mut MockSink {
    &mut self.mock_sink
  }

  /// Receive a message sent to the mock sink.
  pub(crate) async fn receive_message(&mut self) -> Option<Message> {
    self.rx.recv().await
  }

  /// Create a valid encoded signaling message for testing.
  pub(crate) fn create_encoded_message(msg: &SignalingMessage) -> Vec<u8> {
    encode_signaling_message(msg).expect("Failed to encode message")
  }

  /// Create an invalid frame for testing error handling.
  pub(crate) fn create_invalid_frame_data() -> Vec<u8> {
    // Invalid magic number
    vec![0x00, 0x00, 0x00]
  }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Create a valid token for testing by registering and logging in a user.
pub(crate) fn create_valid_token(ws_state: &WebSocketState, username: &str) -> String {
  let (_user_id, token) = ws_state
    .user_store
    .register(username, "test_password")
    .expect("Failed to register user");
  token
}

/// Create an invalid token for testing.
pub(crate) fn create_invalid_token() -> String {
  "invalid.token.data".to_string()
}

pub(crate) use crate::auth::UserStore;
pub(crate) use crate::config::Config;
pub(crate) use crate::ws::handler::handle_binary_message;
pub(crate) use crate::ws::{ConnectionState, WebSocketState, encode_signaling_message};
pub(crate) use axum::extract::ws::Message;
pub(crate) use futures::Sink;
pub(crate) use message::signaling::{ConnectionInvite, Pong, SignalingMessage, TokenAuth};
pub(crate) use message::types::UserStatus;
pub(crate) use std::sync::Arc;
pub(crate) use std::task::{Context, Poll};
pub(crate) use tokio::sync::mpsc;
