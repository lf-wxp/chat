//! WebSocket module tests.

mod chunking;
mod connection;
mod discovery;
mod heartbeat;
mod malformed;
mod message;
mod reconnection;
mod timeout;

pub(super) use super::*;
pub(super) use crate::auth::UserStore;
pub(super) use ::message::frame::{
  CHUNKING_THRESHOLD, MAX_REASSEMBLY_BUFFERS, REASSEMBLY_TIMEOUT_SECS, chunk_message,
};
pub(super) use ::message::signaling::SessionInvalidated;
pub(super) use ::message::types::RoomType;
pub(super) use ::message::{
  ChunkManager, ChunkedMessage, ConnectionInvite, CreateRoom, MAX_CHUNK_SIZE, MessageFrame, Pong,
  TokenAuth, UserId, decode_frame, encode_frame,
};

pub(super) fn create_test_config() -> Config {
  Config::default()
}

pub(super) fn create_test_ws_state() -> WebSocketState {
  let config = create_test_config();
  let user_store = UserStore::new(&config);
  WebSocketState::new(config, user_store)
}

pub(super) fn create_test_sender() -> mpsc::Sender<Vec<u8>> {
  let (tx, _rx) = mpsc::channel(16);
  tx
}
