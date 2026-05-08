use super::*;

pub(super) fn create_test_state() -> DiscoveryState {
  DiscoveryState::new()
}

mod active_peers;
mod edge_cases;
mod invitation;
mod sdp_negotiation;
