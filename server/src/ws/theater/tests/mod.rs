//! Unit tests for theater mode state preconditions.
//!
//! These tests verify room state setup and preconditions (ownership, membership, etc.)
//! that the handler functions rely on. The actual handler function invocation
//! (including error responses, message broadcasting, and permission checks) is
//! thoroughly tested in `server/tests/integration_theater.rs` via real WebSocket
//! connections, covering:
//! - SIG301/SIG311: Room not found
//! - SIG302/SIG312: Non-owner rejection
//! - SIG313: Self-transfer rejection
//! - SIG314: Target not a member
//! - Successful mute-all broadcast (excludes sender)
//! - Successful transfer broadcast (includes sender)

mod concurrent;
mod edge_cases;
mod mute_all;
mod room_type;
mod transfer_owner;

use super::*;
use crate::ws::tests::{create_test_sender, create_test_ws_state};
use message::signaling::{CreateRoom, JoinRoom, TheaterMuteAll, TheaterTransferOwner};
use message::types::RoomType;
