//! Unit tests for invitation handling functions.
//!
//! These tests verify room state setup and preconditions (existence, membership)
//! that the handler functions rely on. The actual handler function invocation
//! (including error responses, message broadcasting, and permission checks) is
//! thoroughly tested in `server/tests/integration_invite.rs` via real WebSocket
//! connections.

mod connection_invite;
mod invite_accepted;
mod invite_declined;
mod invite_timeout;
mod multi_invite;

use super::*;
use crate::ws::tests::{create_test_sender, create_test_ws_state};
use message::signaling::{ConnectionInvite, MultiInvite};
