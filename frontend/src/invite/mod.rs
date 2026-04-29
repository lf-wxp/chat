//! Connection-invite manager (Req 9.1 — 9.14).
//!
//! Tracks the lifecycle of every WebRTC connection invitation that the
//! local user has sent or received and exposes reactive Leptos signals
//! for the UI layer:
//!
//! - **Outbound invites**: pending state per target so the "Send" button
//!   can switch to "Inviting…" and back automatically when the target
//!   accepts, declines, times out or the local user cancels.
//! - **Inbound invites**: a small queue of unanswered invitations
//!   surfaced to the [`crate::components::IncomingInviteModal`].
//! - **Multi-invite tracking**: while a multi-invite is outstanding, all
//!   selected targets are individually tracked so the UI can render a
//!   per-target status row, and the manager fires a
//!   "No one accepted the invitation" resolution once every member of
//!   the batch has been settled (Req 9.12).
//!
//! All persistence is in-memory only — the requirements explicitly say
//! invitations are ephemeral and must not survive a refresh (Req 9.8 —
//! 60 s timeout, Req 9.9 — duplicate invitation guard). The 60 s
//! per-invite timeout is enforced client-side so the UI does not depend
//! on the server's timeout broadcast for state cleanup.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use leptos::prelude::*;
use message::UserId;

use crate::utils::IntervalHandle;
#[cfg(target_arch = "wasm32")]
use crate::utils::set_interval;
use crate::wasm_send_sync;

mod models;
#[cfg(test)]
mod tests;

pub use models::{BatchProgress, IncomingInvite, InviteStatus, OutboundInvite};

/// Default per-invite timeout in milliseconds (Req 9.8).
pub const INVITE_TIMEOUT_MS: i64 = 60_000;

/// Cleanup tick interval in milliseconds — drives both timeout sweeps
/// and stale-state pruning. 1 s is a reasonable trade-off between UI
/// responsiveness (countdowns refresh every second in the UI) and
/// timer overhead.
pub const CLEANUP_INTERVAL_MS: i32 = 1_000;

/// Outcome of resolving an outbound invite.
///
/// Returned by [`InviteManager::accept_outbound`],
/// [`InviteManager::decline_outbound`] and
/// [`InviteManager::timeout_outbound`] so callers can decide how to
/// surface the resolution to the user (e.g. info toast on decline /
/// timeout per Req 9.7 / 9.8) and detect when a multi-invite batch has
/// finished without acceptance (Req 9.12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveOutcome {
  /// The outbound invite that was just removed (or transitioned) from
  /// the pending map.
  pub invite: OutboundInvite,
  /// `Some` when the resolution caused the parent batch to fully
  /// settle. The UI uses this to fire the "No one accepted" toast
  /// when [`BatchProgress::is_unanswered`] returns true.
  pub batch_completed: Option<BatchProgress>,
}

/// Type alias for the cleanup tick observer — fires once per tick that
/// produced at least one resolution. Boxed so multiple observers don't
/// have to share a generic.
pub type TickObserver = Rc<dyn Fn(&CleanupOutcome)>;

/// Reactive invite state shared across UI surfaces.
#[derive(Clone)]
pub struct InviteManager {
  /// Outstanding outbound invitations keyed by target user id.
  outbound: RwSignal<HashMap<UserId, OutboundInvite>>,
  /// Queue of unanswered inbound invitations, oldest first.
  inbound: RwSignal<Vec<IncomingInvite>>,
  /// Per-batch progress tracker for multi-invites (Req 9.12). Entries
  /// are removed eagerly once [`BatchProgress::is_complete`] returns
  /// true so memory does not grow unbounded.
  batches: RwSignal<HashMap<uuid::Uuid, BatchProgress>>,
  /// Non-reactive plumbing (cleanup interval handle + tick observer).
  inner: Rc<RefCell<InviteInner>>,
}

struct InviteInner {
  cleanup: Option<IntervalHandle>,
  tick_observer: Option<TickObserver>,
}

wasm_send_sync!(InviteManager);

impl InviteManager {
  /// Create a new invite manager and start the cleanup interval.
  #[must_use]
  pub fn new() -> Self {
    let manager = Self {
      outbound: RwSignal::new(HashMap::new()),
      inbound: RwSignal::new(Vec::new()),
      batches: RwSignal::new(HashMap::new()),
      inner: Rc::new(RefCell::new(InviteInner {
        cleanup: None,
        tick_observer: None,
      })),
    };
    // The cleanup loop touches `web_sys::window()` which is only valid
    // inside a browser. Skip it on native builds (unit tests) — those
    // exercise the deterministic [`tick`] entry point instead.
    #[cfg(target_arch = "wasm32")]
    manager.start_cleanup();
    manager
  }

  /// Register an outbound invite as pending.
  ///
  /// Returns `false` when an invite to the same target is already in
  /// flight (Req 9.9 — duplicate invitation guard).
  pub fn track_outbound(&self, target: UserId, display_name: String) -> bool {
    let now = chrono::Utc::now().timestamp_millis();
    let mut inserted = true;
    self.outbound.update(|map| {
      if map.contains_key(&target) {
        inserted = false;
        return;
      }
      map.insert(
        target.clone(),
        OutboundInvite {
          target,
          display_name,
          status: InviteStatus::Pending,
          sent_at_ms: now,
          deadline_ms: now + INVITE_TIMEOUT_MS,
          batch_id: None,
        },
      );
    });
    inserted
  }

  /// Register a batch of multi-invite targets at once. Targets that
  /// already have a pending invite are silently skipped. The returned
  /// `Vec` lists the targets that were freshly added so the caller can
  /// emit signaling messages for exactly those.
  pub fn track_multi_outbound(
    &self,
    targets: Vec<(UserId, String)>,
    batch_id: uuid::Uuid,
  ) -> Vec<UserId> {
    let now = chrono::Utc::now().timestamp_millis();
    let mut added = Vec::new();
    self.outbound.update(|map| {
      for (target, display_name) in targets {
        if map.contains_key(&target) {
          continue;
        }
        added.push(target.clone());
        map.insert(
          target.clone(),
          OutboundInvite {
            target,
            display_name,
            status: InviteStatus::Pending,
            sent_at_ms: now,
            deadline_ms: now + INVITE_TIMEOUT_MS,
            batch_id: Some(batch_id),
          },
        );
      }
    });
    if !added.is_empty() {
      self.batches.update(|map| {
        map.insert(
          batch_id,
          BatchProgress {
            total: added.len(),
            accepted: 0,
            declined: 0,
            timed_out: 0,
          },
        );
      });
    }
    added
  }

  /// Mark the outbound invite for `target` as accepted. The entry is
  /// transitioned to [`InviteStatus::Connecting`] and kept in the map
  /// so the UI can render the "Connection being established, please
  /// wait…" status (Req 9.14). Caller must follow up with
  /// [`Self::clear_outbound`] once the SDP / DataChannel handshake
  /// completes.
  pub fn accept_outbound(&self, target: &UserId) -> Option<ResolveOutcome> {
    let mut taken = None;
    self.outbound.update(|map| {
      if let Some(entry) = map.get_mut(target) {
        entry.status = InviteStatus::Connecting;
        taken = Some(entry.clone());
      }
    });
    let invite = taken?;
    let batch_completed = invite
      .batch_id
      .and_then(|bid| self.bump_batch(bid, BatchEvent::Accepted));
    Some(ResolveOutcome {
      invite,
      batch_completed,
    })
  }

  /// Remove an outbound invite once its WebRTC connection has finished
  /// negotiating (or when the local user closes the connection). Used
  /// to clear `Connecting` entries left behind by
  /// [`Self::accept_outbound`].
  pub fn clear_outbound(&self, target: &UserId) -> Option<OutboundInvite> {
    let mut taken = None;
    self.outbound.update(|map| {
      taken = map.remove(target);
    });
    taken
  }

  /// Mark the outbound invite for `target` as declined. Returns the
  /// removed invite plus any newly-completed batch progress so the
  /// caller can decide how to surface the rejection to the user.
  pub fn decline_outbound(&self, target: &UserId) -> Option<ResolveOutcome> {
    let invite = self.remove_outbound(target)?;
    let batch_completed = invite
      .batch_id
      .and_then(|bid| self.bump_batch(bid, BatchEvent::Declined));
    Some(ResolveOutcome {
      invite,
      batch_completed,
    })
  }

  /// Mark the outbound invite for `target` as timed-out. Returns the
  /// removed invite plus any newly-completed batch progress.
  pub fn timeout_outbound(&self, target: &UserId) -> Option<ResolveOutcome> {
    let invite = self.remove_outbound(target)?;
    let batch_completed = invite
      .batch_id
      .and_then(|bid| self.bump_batch(bid, BatchEvent::TimedOut));
    Some(ResolveOutcome {
      invite,
      batch_completed,
    })
  }

  /// Cancel an outbound invite locally without emitting any signaling
  /// (used when the local user closes the modal before the timeout, or
  /// when sending fails). Cancellation is treated as "the invite never
  /// happened" so the parent batch's `total` is decremented to allow
  /// the remaining members to complete normally.
  pub fn cancel_outbound(&self, target: &UserId) -> Option<OutboundInvite> {
    let invite = self.remove_outbound(target)?;
    if let Some(bid) = invite.batch_id {
      self.batches.update(|map| {
        if let Some(progress) = map.get_mut(&bid) {
          progress.total = progress.total.saturating_sub(1);
          if progress.is_complete() {
            map.remove(&bid);
          }
        }
      });
    }
    Some(invite)
  }

  fn remove_outbound(&self, target: &UserId) -> Option<OutboundInvite> {
    let mut taken = None;
    self.outbound.update(|map| {
      taken = map.remove(target);
    });
    taken
  }

  fn bump_batch(&self, batch_id: uuid::Uuid, event: BatchEvent) -> Option<BatchProgress> {
    let mut completed = None;
    self.batches.update(|map| {
      let Some(progress) = map.get_mut(&batch_id) else {
        return;
      };
      match event {
        BatchEvent::Accepted => progress.accepted += 1,
        BatchEvent::Declined => progress.declined += 1,
        BatchEvent::TimedOut => progress.timed_out += 1,
      }
      if progress.is_complete() {
        completed = Some(progress.clone());
        map.remove(&batch_id);
      }
    });
    completed
  }

  /// Returns `true` when a pending invite to `target` already exists.
  #[must_use]
  pub fn has_pending_outbound(&self, target: &UserId) -> bool {
    self.outbound.with(|map| map.contains_key(target))
  }

  /// Untracked variant of [`Self::has_pending_outbound`] for use
  /// outside the Leptos reactive owner.
  #[must_use]
  pub fn has_pending_outbound_untracked(&self, target: &UserId) -> bool {
    self.outbound.with_untracked(|map| map.contains_key(target))
  }

  /// Returns the current `InviteStatus` for an outbound invite, or
  /// `None` when no invite is in flight. The UI renders different
  /// labels for `Pending` ("Inviting…") vs `Connecting`
  /// ("Connection being established, please wait…").
  #[must_use]
  pub fn outbound_status(&self, target: &UserId) -> Option<InviteStatus> {
    self.outbound.with(|map| map.get(target).map(|i| i.status))
  }

  /// Reactive accessor for the outbound map — used by status-aware UI
  /// elements (e.g. the "Inviting…" button).
  #[must_use]
  pub fn outbound_signal(&self) -> RwSignal<HashMap<UserId, OutboundInvite>> {
    self.outbound
  }

  /// Reactive accessor for the inbound queue — consumed by the
  /// `IncomingInviteModal` component to render the front of the queue.
  #[must_use]
  pub fn inbound_signal(&self) -> RwSignal<Vec<IncomingInvite>> {
    self.inbound
  }

  /// Snapshot of every outbound invite, sorted by oldest first.
  #[must_use]
  pub fn outbound_list(&self) -> Vec<OutboundInvite> {
    let mut list: Vec<OutboundInvite> = self.outbound.with(|map| map.values().cloned().collect());
    list.sort_by_key(|i| i.sent_at_ms);
    list
  }

  /// Append an inbound invite to the queue. Duplicates from the same
  /// inviter are coalesced into the existing entry by refreshing its
  /// `received_at_ms`.
  pub fn push_inbound(&self, invite: IncomingInvite) {
    self.inbound.update(|queue| {
      if let Some(existing) = queue.iter_mut().find(|i| i.from == invite.from) {
        existing.received_at_ms = invite.received_at_ms;
        existing.deadline_ms = invite.deadline_ms;
        existing.note = invite.note.clone();
      } else {
        queue.push(invite);
      }
    });
  }

  /// Pop the inbound invite from `inviter` (if present). Returns the
  /// removed invite so callers can pass its data to the signaling
  /// layer.
  pub fn take_inbound(&self, inviter: &UserId) -> Option<IncomingInvite> {
    let mut taken = None;
    self.inbound.update(|queue| {
      if let Some(idx) = queue.iter().position(|i| i.from == *inviter) {
        taken = Some(queue.remove(idx));
      }
    });
    taken
  }

  /// Returns the front-of-queue inbound invite, if any. Used by the
  /// modal component which only renders one invite at a time.
  #[must_use]
  pub fn front_inbound(&self) -> Option<IncomingInvite> {
    self.inbound.with(|queue| queue.first().cloned())
  }

  /// Tear the manager down: cancel the cleanup interval and clear
  /// in-memory state. After this call the manager will no longer fire
  /// observer notifications. Used by application shutdown / hot
  /// reload paths only — for ordinary logout, prefer
  /// [`Self::clear_state`] which keeps the cleanup loop and observer
  /// alive so the same manager can be reused after re-login.
  pub fn shutdown(&self) {
    let cleanup = self.inner.borrow_mut().cleanup.take();
    if let Some(handle) = cleanup {
      handle.cancel();
    }
    self.outbound.set(HashMap::new());
    self.inbound.set(Vec::new());
    self.batches.set(HashMap::new());
    self.inner.borrow_mut().tick_observer = None;
  }

  /// Drop all in-flight invites without tearing down the cleanup
  /// interval or the tick observer. Intended for the logout path so
  /// the next login starts with an empty queue but reuses the same
  /// manager instance.
  pub fn clear_state(&self) {
    self.outbound.set(HashMap::new());
    self.inbound.set(Vec::new());
    self.batches.set(HashMap::new());
  }

  /// Drive a single cleanup tick — exposed for tests so the timer
  /// behaviour can be exercised without a browser. The function is
  /// pure-Rust (does not touch `web_sys`).
  ///
  /// Each timed-out invite contributes to its parent batch (if any);
  /// the returned [`CleanupOutcome`] also enumerates batches that
  /// completed during the tick so the caller can fire the
  /// "No one accepted the invitation" toast (Req 9.12).
  pub fn tick(&self, now_ms: i64) -> CleanupOutcome {
    let mut outcome = CleanupOutcome::default();

    let mut expired_outbound: Vec<OutboundInvite> = Vec::new();
    self.outbound.update(|map| {
      map.retain(|_, invite| {
        // `Connecting` invites are not expired by the watchdog —
        // the SDP layer is responsible for clearing them via
        // `clear_outbound` once the negotiation finishes.
        if invite.status == InviteStatus::Connecting {
          return true;
        }
        if invite.deadline_ms <= now_ms {
          expired_outbound.push(invite.clone());
          false
        } else {
          true
        }
      });
    });
    for invite in expired_outbound {
      outcome.outbound_timed_out.push(invite.target.clone());
      if let Some(bid) = invite.batch_id
        && let Some(progress) = self.bump_batch(bid, BatchEvent::TimedOut)
      {
        outcome.batches_completed.push((bid, progress));
      }
    }

    self.inbound.update(|queue| {
      queue.retain(|invite| {
        if invite.deadline_ms <= now_ms {
          outcome.inbound_timed_out.push(invite.from.clone());
          false
        } else {
          true
        }
      });
    });

    outcome
  }

  /// Install a callback that fires once per cleanup tick whose
  /// outcome is non-empty. Used by the signaling layer to surface
  /// "Invitation has timed out" toasts and the "No one accepted"
  /// multi-invite resolution. Replaces any previously-installed
  /// observer.
  pub fn set_tick_observer<F>(&self, observer: F)
  where
    F: Fn(&CleanupOutcome) + 'static,
  {
    self.inner.borrow_mut().tick_observer = Some(Rc::new(observer));
  }

  /// Fire the cleanup observer when the outcome contains at least one
  /// transition. Called from the WASM `setInterval` loop and from
  /// unit tests after a manual [`Self::tick`].
  #[cfg(any(test, target_arch = "wasm32"))]
  pub(crate) fn notify_observer(&self, outcome: &CleanupOutcome) {
    if outcome.is_empty() {
      return;
    }
    let observer = self.inner.borrow().tick_observer.clone();
    if let Some(observer) = observer {
      observer(outcome);
    }
  }

  #[cfg(target_arch = "wasm32")]
  fn start_cleanup(&self) {
    let manager = self.clone();
    let handle = set_interval(CLEANUP_INTERVAL_MS, move || {
      let now = chrono::Utc::now().timestamp_millis();
      let outcome = manager.tick(now);
      manager.notify_observer(&outcome);
    });
    if let Some(handle) = handle {
      self.inner.borrow_mut().cleanup = Some(handle);
    }
  }
}

impl Default for InviteManager {
  fn default() -> Self {
    Self::new()
  }
}

#[derive(Copy, Clone, Debug)]
enum BatchEvent {
  Accepted,
  Declined,
  TimedOut,
}

/// Outcome returned by [`InviteManager::tick`] for tests / diagnostics.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CleanupOutcome {
  /// Outbound invites whose deadline elapsed during this tick.
  pub outbound_timed_out: Vec<UserId>,
  /// Inbound invites whose deadline elapsed during this tick.
  pub inbound_timed_out: Vec<UserId>,
  /// Multi-invite batches that completed during this tick (e.g. the
  /// last pending invite expired). The UI fires the
  /// "No one accepted the invitation" toast for batches whose
  /// [`BatchProgress::is_unanswered`] is true.
  pub batches_completed: Vec<(uuid::Uuid, BatchProgress)>,
}

impl CleanupOutcome {
  /// Returns `true` when no outbound / inbound / batch transitions
  /// occurred this tick.
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.outbound_timed_out.is_empty()
      && self.inbound_timed_out.is_empty()
      && self.batches_completed.is_empty()
  }
}

/// Provide an `InviteManager` to the Leptos context.
pub fn provide_invite_manager() -> InviteManager {
  let manager = InviteManager::new();
  provide_context(manager.clone());
  manager
}

/// Retrieve the `InviteManager` from Leptos context.
///
/// # Panics
/// Panics if `provide_invite_manager` has not been called.
#[must_use]
pub fn use_invite_manager() -> InviteManager {
  expect_context::<InviteManager>()
}

/// Best-effort accessor — safe to call from non-reactive callbacks.
#[must_use]
pub fn try_use_invite_manager() -> Option<InviteManager> {
  use_context::<InviteManager>()
}
