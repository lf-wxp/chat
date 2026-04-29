//! Plain-data types describing invitations tracked by the
//! `InviteManager`. Kept in their own module so they can be unit-tested
//! and reused from UI components without pulling in the full manager.

use message::UserId;

/// Lifecycle of an outbound invite.
///
/// `Pending` is the only "live" state held in the outbound map — once
/// the invite resolves the entry is removed and the resolution surfaces
/// through the [`crate::invite::CleanupOutcome`] /
/// [`crate::invite::ResolveOutcome`] return types so the UI layer can
/// toast the result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InviteStatus {
  /// The invite has been sent and is awaiting a response.
  Pending,
  /// The invitee accepted but the local SDP negotiation has not yet
  /// completed (Req 9.14). Surfaces as the "Connecting, please wait…"
  /// status in the UI for as long as the entry remains in the outbound
  /// map.
  Connecting,
}

/// Outbound invite tracked locally for UI feedback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboundInvite {
  /// Target user id.
  pub target: UserId,
  /// Cached display name at the time of sending — used by the UI even
  /// when the target later goes offline.
  pub display_name: String,
  /// Current lifecycle status.
  pub status: InviteStatus,
  /// Unix-ms timestamp at which the invite was sent.
  pub sent_at_ms: i64,
  /// Unix-ms deadline after which the invite is auto-expired.
  pub deadline_ms: i64,
  /// `Some` for invites that are part of a multi-invite batch — used
  /// to display a per-batch progress indicator in the UI and to fire
  /// the "no one accepted" toast (Req 9.12).
  pub batch_id: Option<uuid::Uuid>,
}

/// Incoming invite waiting for the user to accept or decline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomingInvite {
  /// User id of the inviter.
  pub from: UserId,
  /// Cached display name of the inviter for offline-capable rendering.
  pub display_name: String,
  /// Optional note attached to the invitation.
  pub note: Option<String>,
  /// Unix-ms timestamp of when the invite was received.
  pub received_at_ms: i64,
  /// Unix-ms deadline after which the invite expires locally.
  pub deadline_ms: i64,
}

impl IncomingInvite {
  /// Convenience constructor for tests and signaling-layer adapters.
  #[must_use]
  pub fn new(
    from: UserId,
    display_name: String,
    note: Option<String>,
    received_at_ms: i64,
    timeout_ms: i64,
  ) -> Self {
    Self {
      from,
      display_name,
      note,
      received_at_ms,
      deadline_ms: received_at_ms + timeout_ms,
    }
  }
}

/// Per-batch progress tracker for multi-invite (Req 9.12).
///
/// The manager keeps one entry per `batch_id` while at least one
/// outbound invite in the batch is still pending. Once `accepted +
/// declined + timed_out == total` the manager removes the entry and
/// signals the resolution to the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchProgress {
  /// Total number of invites in this batch (including ones that may
  /// have been suppressed at track-time because of duplicates — they
  /// are still counted so the math adds up).
  pub total: usize,
  /// Count of invites whose recipient accepted.
  pub accepted: usize,
  /// Count of invites whose recipient explicitly declined.
  pub declined: usize,
  /// Count of invites that the local timeout sweeper expired.
  pub timed_out: usize,
}

impl BatchProgress {
  /// Returns `true` once every member of the batch has resolved
  /// (accepted, declined or timed out).
  #[must_use]
  pub fn is_complete(&self) -> bool {
    self.accepted + self.declined + self.timed_out >= self.total
  }

  /// Returns `true` when the batch has resolved without a single
  /// acceptance — used to fire the "No one accepted" toast.
  #[must_use]
  pub fn is_unanswered(&self) -> bool {
    self.is_complete() && self.accepted == 0
  }
}
