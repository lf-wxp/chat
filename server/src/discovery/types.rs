//! Discovery-related types and error definitions.

use std::time::{Duration, Instant};

use message::RoomId;
use message::UserId;

// =============================================================================
// Rate Limiting Constants
// =============================================================================

/// Maximum invitations per user per minute.
pub const INVITE_RATE_LIMIT_PER_MINUTE: usize = 10;
/// Maximum invitations per user per hour.
pub const INVITE_RATE_LIMIT_PER_HOUR: usize = 50;
/// Maximum unanswered invitations per target user.
pub const MAX_UNANSWERED_INVITATIONS_PER_TARGET: usize = 5;
/// Invitation timeout duration (60 seconds).
pub const INVITATION_TIMEOUT: Duration = Duration::from_secs(60);
/// SDP negotiation timeout duration (30 seconds).
pub const SDP_NEGOTIATION_TIMEOUT_SECS: u64 = 30;

// =============================================================================
// Invitation Types
// =============================================================================

/// Unique invitation identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InvitationId(String);

impl InvitationId {
  /// Generate a new unique invitation ID.
  #[must_use]
  pub fn new() -> Self {
    Self(uuid::Uuid::new_v4().to_string())
  }
}

impl Default for InvitationId {
  fn default() -> Self {
    Self::new()
  }
}

impl std::fmt::Display for InvitationId {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

/// Pending invitation record.
#[derive(Debug, Clone)]
pub struct PendingInvitation {
  /// Invitation ID (for tracking).
  pub id: InvitationId,
  /// Who sent the invitation.
  pub from: UserId,
  /// Who receives the invitation.
  pub to: UserId,
  /// Optional note from inviter.
  pub note: Option<String>,
  /// When the invitation was created.
  pub created_at: Instant,
  /// For multi-user invitations, the associated room ID (if any).
  pub room_id: Option<RoomId>,
}

impl PendingInvitation {
  /// Create a new pending invitation.
  #[must_use]
  pub fn new(from: UserId, to: UserId, note: Option<String>) -> Self {
    Self {
      id: InvitationId::new(),
      from,
      to,
      note,
      created_at: Instant::now(),
      room_id: None,
    }
  }

  /// Create a pending invitation with room ID (for multi-user invitations).
  #[must_use]
  pub fn with_room(from: UserId, to: UserId, note: Option<String>, room_id: RoomId) -> Self {
    Self {
      id: InvitationId::new(),
      from,
      to,
      note,
      created_at: Instant::now(),
      room_id: Some(room_id),
    }
  }

  /// Check if the invitation has timed out.
  #[must_use]
  pub fn is_timed_out(&self) -> bool {
    self.created_at.elapsed() > INVITATION_TIMEOUT
  }
}

// =============================================================================
// Error Types
// =============================================================================

/// Error types for invitation operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvitationError {
  /// Rate limit exceeded.
  RateLimitExceeded,
  /// Already have a pending invitation to this user.
  AlreadyPending,
  /// Target has too many unanswered invitations.
  TargetLimitExceeded,
  /// No valid targets for multi-invite.
  NoValidTargets,
  /// Invitation not found.
  NotFound,
}

impl std::fmt::Display for InvitationError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::RateLimitExceeded => write!(f, "Rate limit exceeded"),
      Self::AlreadyPending => write!(f, "Invitation already pending"),
      Self::TargetLimitExceeded => write!(f, "Target has too many unanswered invitations"),
      Self::NoValidTargets => write!(f, "No valid targets for invitation"),
      Self::NotFound => write!(f, "Invitation not found"),
    }
  }
}

impl std::error::Error for InvitationError {}

// =============================================================================
// Multi-Invite Types
// =============================================================================

/// State for a multi-user invitation.
#[derive(Debug, Clone)]
pub struct MultiInviteState {
  /// Who sent the invitation.
  pub from: UserId,
  /// Target user IDs.
  pub targets: Vec<UserId>,
  /// Room ID (created when first user accepts).
  pub room_id: Option<RoomId>,
  /// Users who have accepted.
  pub accepted: Vec<UserId>,
  /// Users who have declined or timed out.
  pub declined: Vec<UserId>,
  /// When the invitation was created.
  pub created_at: Instant,
}

impl MultiInviteState {
  /// Create a new multi-invite state.
  #[must_use]
  pub fn new(from: UserId, targets: Vec<UserId>) -> Self {
    Self {
      from,
      targets,
      room_id: None,
      accepted: Vec::new(),
      declined: Vec::new(),
      created_at: Instant::now(),
    }
  }

  /// Check if the multi-invite has timed out.
  #[must_use]
  pub fn is_timed_out(&self) -> bool {
    self.created_at.elapsed() > INVITATION_TIMEOUT
  }

  /// Check if all targets have responded.
  #[must_use]
  pub fn is_complete(&self) -> bool {
    let responded_count = self.accepted.len() + self.declined.len();
    responded_count >= self.targets.len()
  }

  /// Get remaining targets who haven't responded.
  #[must_use]
  pub fn remaining_targets(&self) -> Vec<UserId> {
    self
      .targets
      .iter()
      .filter(|t| !self.accepted.contains(t) && !self.declined.contains(t))
      .cloned()
      .collect()
  }
}

/// Result of accepting a multi-user invitation.
#[derive(Debug, Clone)]
pub enum MultiInviteAcceptResult {
  /// First user accepted, room was just created.
  FirstAcceptance {
    /// The newly created room ID.
    room_id: RoomId,
    /// Remaining targets who haven't responded.
    remaining_targets: Vec<UserId>,
  },
  /// Subsequent user accepted, should join existing room.
  JoinRoom {
    /// The existing room ID to join.
    room_id: RoomId,
  },
}

/// Statistics for a multi-user invitation.
#[derive(Debug, Clone)]
pub struct MultiInviteStats {
  /// Who sent the invitation.
  pub from: UserId,
  /// Total number of targets.
  pub total_targets: usize,
  /// Number of accepted invitations.
  pub accepted: usize,
  /// Number of declined invitations.
  pub declined: usize,
  /// Room ID if created.
  pub room_id: Option<RoomId>,
}

// =============================================================================
// SDP Negotiation Types
// =============================================================================

/// SDP negotiation state.
#[derive(Debug, Clone)]
pub struct SdpNegotiationState {
  /// Who initiated the SDP negotiation.
  pub initiator: UserId,
  /// Who is the target of the negotiation.
  pub target: UserId,
  /// Whether an SDP offer has been sent.
  pub offer_sent: bool,
  /// Whether an SDP answer has been received.
  pub answer_received: bool,
  /// When the negotiation started.
  pub started_at: Instant,
}

impl SdpNegotiationState {
  /// Create a new SDP negotiation state.
  #[must_use]
  pub fn new(initiator: UserId, target: UserId) -> Self {
    Self {
      initiator,
      target,
      offer_sent: false,
      answer_received: false,
      started_at: Instant::now(),
    }
  }

  /// Check if the negotiation is in progress.
  /// A negotiation is in progress if it exists and the answer has not been received yet.
  #[must_use]
  pub fn is_in_progress(&self) -> bool {
    !self.answer_received
  }

  /// Check if the negotiation is complete.
  #[must_use]
  pub fn is_complete(&self) -> bool {
    self.offer_sent && self.answer_received
  }

  /// Check if the negotiation has timed out (30 seconds).
  #[must_use]
  pub fn is_timed_out(&self) -> bool {
    self.started_at.elapsed() > Duration::from_secs(SDP_NEGOTIATION_TIMEOUT_SECS)
  }
}
