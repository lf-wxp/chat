//! Unit tests for the invite manager state machine.
//!
//! The tests run in a Leptos `Owner` so the internal `RwSignal`s have
//! a reactive runtime; the cleanup interval is never started here
//! because `set_interval` is a no-op outside the browser.

use super::{CleanupOutcome, INVITE_TIMEOUT_MS, IncomingInvite, InviteManager, InviteStatus};
use leptos::prelude::*;
use message::UserId;

fn user(seed: &str) -> UserId {
  UserId::from_uuid(uuid::Uuid::new_v5(
    &uuid::Uuid::NAMESPACE_DNS,
    seed.as_bytes(),
  ))
}

fn with_runtime<F: FnOnce()>(f: F) {
  let owner = Owner::new();
  owner.with(f);
}

#[test]
fn outbound_track_records_pending_status() {
  with_runtime(|| {
    let manager = InviteManager::new();
    let target = user("alice");
    assert!(manager.track_outbound(target.clone(), "Alice".into()));
    assert!(manager.has_pending_outbound(&target));
    let invite = manager
      .outbound_signal()
      .get()
      .get(&target)
      .cloned()
      .expect("invite present");
    assert_eq!(invite.status, InviteStatus::Pending);
    assert_eq!(invite.display_name, "Alice");
    assert_eq!(invite.deadline_ms - invite.sent_at_ms, INVITE_TIMEOUT_MS);
    manager.shutdown();
  });
}

#[test]
fn outbound_duplicate_invite_is_rejected() {
  with_runtime(|| {
    let manager = InviteManager::new();
    let target = user("bob");
    assert!(manager.track_outbound(target.clone(), "Bob".into()));
    assert!(!manager.track_outbound(target, "Bob".into()));
    manager.shutdown();
  });
}

#[test]
fn outbound_accept_transitions_to_connecting() {
  with_runtime(|| {
    let manager = InviteManager::new();
    let target = user("carol");
    manager.track_outbound(target.clone(), "Carol".into());
    let outcome = manager.accept_outbound(&target).expect("present");
    assert_eq!(outcome.invite.status, InviteStatus::Connecting);
    // Entry must remain so the UI can render the connecting status.
    assert!(manager.has_pending_outbound(&target));
    assert_eq!(
      manager.outbound_status(&target),
      Some(InviteStatus::Connecting)
    );
    manager.shutdown();
  });
}

#[test]
fn clear_outbound_removes_connecting_entry() {
  with_runtime(|| {
    let manager = InviteManager::new();
    let target = user("carla");
    manager.track_outbound(target.clone(), "Carla".into());
    manager.accept_outbound(&target);
    let removed = manager.clear_outbound(&target).expect("present");
    assert_eq!(removed.target, target);
    assert!(!manager.has_pending_outbound(&target));
    manager.shutdown();
  });
}

#[test]
fn outbound_decline_returns_invite_metadata() {
  with_runtime(|| {
    let manager = InviteManager::new();
    let target = user("dave");
    manager.track_outbound(target.clone(), "Dave".into());
    let outcome = manager.decline_outbound(&target).expect("present");
    assert_eq!(outcome.invite.target, target);
    assert_eq!(outcome.invite.display_name, "Dave");
    assert!(outcome.batch_completed.is_none());
    assert!(!manager.has_pending_outbound(&target));
    manager.shutdown();
  });
}

#[test]
fn outbound_cancel_returns_invite() {
  with_runtime(|| {
    let manager = InviteManager::new();
    let target = user("eve");
    manager.track_outbound(target.clone(), "Eve".into());
    assert!(manager.cancel_outbound(&target).is_some());
    assert!(!manager.has_pending_outbound(&target));
    manager.shutdown();
  });
}

#[test]
fn multi_outbound_skips_existing_targets() {
  with_runtime(|| {
    let manager = InviteManager::new();
    let alice = user("alice");
    let bob = user("bob");
    manager.track_outbound(alice.clone(), "Alice".into());
    let added = manager.track_multi_outbound(
      vec![(alice.clone(), "Alice".into()), (bob.clone(), "Bob".into())],
      uuid::Uuid::new_v4(),
    );
    assert_eq!(added, vec![bob]);
    manager.shutdown();
  });
}

#[test]
fn batch_completes_when_all_targets_decline() {
  with_runtime(|| {
    let manager = InviteManager::new();
    let alice = user("alice");
    let bob = user("bob");
    let batch = uuid::Uuid::new_v4();
    let added = manager.track_multi_outbound(
      vec![(alice.clone(), "A".into()), (bob.clone(), "B".into())],
      batch,
    );
    assert_eq!(added.len(), 2);
    let first = manager.decline_outbound(&alice).expect("present");
    assert!(first.batch_completed.is_none(), "batch still has bob open");
    let second = manager.decline_outbound(&bob).expect("present");
    let progress = second.batch_completed.expect("batch resolved");
    assert!(progress.is_complete());
    assert!(progress.is_unanswered());
    assert_eq!(progress.declined, 2);
    assert_eq!(progress.accepted, 0);
    manager.shutdown();
  });
}

#[test]
fn batch_with_acceptance_is_not_unanswered() {
  with_runtime(|| {
    let manager = InviteManager::new();
    let alice = user("alice");
    let bob = user("bob");
    let batch = uuid::Uuid::new_v4();
    manager.track_multi_outbound(
      vec![(alice.clone(), "A".into()), (bob.clone(), "B".into())],
      batch,
    );
    manager.accept_outbound(&alice);
    let outcome = manager.timeout_outbound(&bob).expect("present");
    let progress = outcome.batch_completed.expect("batch resolved");
    assert!(progress.is_complete());
    assert!(!progress.is_unanswered());
    assert_eq!(progress.accepted, 1);
    assert_eq!(progress.timed_out, 1);
    manager.shutdown();
  });
}

#[test]
fn cancelled_batch_member_does_not_block_completion() {
  with_runtime(|| {
    let manager = InviteManager::new();
    let alice = user("alice");
    let bob = user("bob");
    let batch = uuid::Uuid::new_v4();
    manager.track_multi_outbound(
      vec![(alice.clone(), "A".into()), (bob.clone(), "B".into())],
      batch,
    );
    // Local cancellation removes alice from the batch (total 2 -> 1).
    manager.cancel_outbound(&alice);
    let outcome = manager.decline_outbound(&bob).expect("present");
    let progress = outcome.batch_completed.expect("batch resolved");
    assert_eq!(progress.total, 1);
    assert!(progress.is_unanswered());
    manager.shutdown();
  });
}

#[test]
fn inbound_push_and_take_round_trip() {
  with_runtime(|| {
    let manager = InviteManager::new();
    let inviter = user("frank");
    let invite = IncomingInvite::new(
      inviter.clone(),
      "Frank".into(),
      Some("hello".into()),
      1_000,
      INVITE_TIMEOUT_MS,
    );
    manager.push_inbound(invite.clone());
    let front = manager.front_inbound().expect("front exists");
    assert_eq!(front.from, inviter);
    let taken = manager.take_inbound(&inviter).expect("taken");
    assert_eq!(taken.note.as_deref(), Some("hello"));
    assert!(manager.front_inbound().is_none());
    manager.shutdown();
  });
}

#[test]
fn inbound_duplicate_is_coalesced() {
  with_runtime(|| {
    let manager = InviteManager::new();
    let inviter = user("grace");
    let first = IncomingInvite::new(
      inviter.clone(),
      "Grace".into(),
      None,
      1_000,
      INVITE_TIMEOUT_MS,
    );
    let second = IncomingInvite::new(
      inviter.clone(),
      "Grace".into(),
      Some("note".into()),
      2_000,
      INVITE_TIMEOUT_MS,
    );
    manager.push_inbound(first);
    manager.push_inbound(second);
    let queue = manager.inbound_signal().get();
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].received_at_ms, 2_000);
    assert_eq!(queue[0].note.as_deref(), Some("note"));
    manager.shutdown();
  });
}

#[test]
fn tick_expires_outbound_and_inbound_invites() {
  with_runtime(|| {
    let manager = InviteManager::new();
    let alice = user("alice");
    let bob = user("bob");
    manager.track_outbound(alice.clone(), "Alice".into());
    manager.push_inbound(IncomingInvite::new(
      bob.clone(),
      "Bob".into(),
      None,
      0,
      INVITE_TIMEOUT_MS,
    ));
    // Pick a `now` that is well past every deadline that the manager
    // could have produced (track_outbound stamps with Utc::now()).
    let future = chrono::Utc::now().timestamp_millis() + INVITE_TIMEOUT_MS * 2;
    let outcome = manager.tick(future);
    assert_eq!(outcome.outbound_timed_out, vec![alice]);
    assert_eq!(outcome.inbound_timed_out, vec![bob]);
    manager.shutdown();
  });
}

#[test]
fn tick_does_not_expire_connecting_invites() {
  with_runtime(|| {
    let manager = InviteManager::new();
    let target = user("hank");
    manager.track_outbound(target.clone(), "Hank".into());
    manager.accept_outbound(&target); // -> Connecting
    let future = chrono::Utc::now().timestamp_millis() + INVITE_TIMEOUT_MS * 2;
    let outcome = manager.tick(future);
    assert!(outcome.outbound_timed_out.is_empty());
    assert!(manager.has_pending_outbound(&target));
    manager.shutdown();
  });
}

#[test]
fn tick_observer_receives_resolution() {
  with_runtime(|| {
    use std::cell::RefCell;
    use std::rc::Rc;

    let manager = InviteManager::new();
    let captured: Rc<RefCell<Option<CleanupOutcome>>> = Rc::new(RefCell::new(None));
    let captured_for_cb = Rc::clone(&captured);
    manager.set_tick_observer(move |outcome| {
      *captured_for_cb.borrow_mut() = Some(outcome.clone());
    });
    let target = user("ivan");
    manager.track_outbound(target.clone(), "Ivan".into());
    let future = chrono::Utc::now().timestamp_millis() + INVITE_TIMEOUT_MS * 2;
    let outcome = manager.tick(future);
    manager.notify_observer(&outcome);
    let snapshot = captured.borrow().as_ref().cloned().expect("observer fired");
    assert_eq!(snapshot.outbound_timed_out, vec![target]);
    manager.shutdown();
  });
}
