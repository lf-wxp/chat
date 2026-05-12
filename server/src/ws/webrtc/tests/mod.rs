//! Unit tests for WebRTC signaling handling functions.
//!
//! These tests verify state preconditions (connection status, peer relationships,
//! SDP negotiation state) that the handler functions rely on. The actual handler
//! function invocation (including error responses and message forwarding) is
//! thoroughly tested in `server/tests/integration_webrtc.rs` via real WebSocket
//! connections.

mod concurrent;
mod ice_candidate;
mod peer_closed;
mod peer_established;
mod sdp_answer;
mod sdp_offer;

use super::*;
use crate::ws::tests::{create_test_sender, create_test_ws_state};
use message::signaling::{IceCandidate, PeerClosed, SdpOffer};
