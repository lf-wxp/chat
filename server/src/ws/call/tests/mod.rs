//! Unit tests for call management handling functions.
//!
//! These tests verify room state setup and preconditions (existence, membership)
//! that the handler functions rely on. The actual handler function invocation
//! (including error responses, message broadcasting, and permission checks) is
//! thoroughly tested in `server/tests/integration_call.rs` via real WebSocket
//! connections.

mod broadcast;
mod call_accept;
mod call_decline;
mod call_end;
mod call_invite;
mod concurrent;

use super::*;
use crate::ws::tests::{create_test_sender, create_test_ws_state};
use message::signaling::{CallAccept, CallDecline, CallEnd, CallInvite, CreateRoom};
use message::types::RoomType;
