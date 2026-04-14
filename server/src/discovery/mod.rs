//! User discovery and connection invitation system.
//!
//! This module provides:
//! - Online user list management
//! - Connection invitation flow (invite/accept/decline/timeout)
//! - Rate limiting for invitations
//! - Multi-user invitation support
//! - Bidirectional invitation conflict detection and merging

mod rate_limit;
mod state;
mod types;

// Re-export public types
pub use rate_limit::UserRateLimit;
pub use state::DiscoveryState;
pub use types::{
  INVITATION_TIMEOUT, INVITE_RATE_LIMIT_PER_HOUR, INVITE_RATE_LIMIT_PER_MINUTE, InvitationError,
  InvitationId, MAX_UNANSWERED_INVITATIONS_PER_TARGET, MultiInviteAcceptResult, MultiInviteState,
  MultiInviteStats, PendingInvitation, SdpNegotiationState,
};

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests;
