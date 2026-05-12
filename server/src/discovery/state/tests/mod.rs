//! Tests for discovery state manager.

mod active_peers;
mod bidirectional;
mod creation;
mod invitation;
mod multi_invite;
mod rate_limit;
mod sdp_negotiation;
mod target_limit;

use super::*;
use crate::discovery::{INVITE_RATE_LIMIT_PER_HOUR, INVITE_RATE_LIMIT_PER_MINUTE};

fn create_invite(from: UserId, to: UserId) -> ConnectionInvite {
  ConnectionInvite {
    from: from.clone(),
    to: to.clone(),
    note: Some("Let's chat!".to_string()),
  }
}

fn create_multi_invite(from: UserId, targets: Vec<UserId>) -> MultiInvite {
  MultiInvite { from, targets }
}
