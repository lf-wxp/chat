//! Unit tests for the blacklist module.
//!
//! These tests intentionally avoid touching `localStorage` — they
//! exercise the pure-Rust state machine and the auto-decline delay
//! helper. The persistence path is covered by the WASM smoke test in
//! `frontend/tests/`.

use super::{
  AUTO_DECLINE_MAX_MS, AUTO_DECLINE_MIN_MS, BlacklistEntry, BlacklistState,
  auto_decline_delay_from_seed,
};
use leptos::prelude::*;
use message::UserId;

fn make_entry(seed: &str) -> BlacklistEntry {
  BlacklistEntry {
    user_id: UserId::from_uuid(uuid::Uuid::new_v5(
      &uuid::Uuid::NAMESPACE_DNS,
      seed.as_bytes(),
    )),
    display_name: seed.to_string(),
    blocked_at_ms: 1_700_000_000_000,
  }
}

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
fn auto_decline_delay_lower_bound() {
  assert_eq!(auto_decline_delay_from_seed(0.0), AUTO_DECLINE_MIN_MS);
}

#[test]
fn auto_decline_delay_upper_bound() {
  // The seed of exactly 1.0 maps to MIN + (MAX-MIN)*1.0 = MAX.
  assert_eq!(auto_decline_delay_from_seed(1.0), AUTO_DECLINE_MAX_MS);
}

#[test]
fn auto_decline_delay_clamps_negative_seed() {
  assert_eq!(auto_decline_delay_from_seed(-0.5), AUTO_DECLINE_MIN_MS);
}

#[test]
fn auto_decline_delay_clamps_seed_above_one() {
  assert_eq!(auto_decline_delay_from_seed(2.0), AUTO_DECLINE_MAX_MS);
}

#[test]
fn auto_decline_delay_midpoint() {
  let mid = auto_decline_delay_from_seed(0.5);
  assert!((45_000..=46_000).contains(&mid), "expected ~45s, got {mid}");
}

#[test]
fn auto_decline_delay_just_below_one_stays_inside_range() {
  let near_max = auto_decline_delay_from_seed(0.9999);
  assert!(near_max <= AUTO_DECLINE_MAX_MS);
  assert!(near_max >= AUTO_DECLINE_MAX_MS - 100);
}

#[test]
fn entry_serialization_roundtrip() {
  let entry = make_entry("alice");
  let json = serde_json::to_string(&entry).expect("serialize");
  let restored: BlacklistEntry = serde_json::from_str(&json).expect("deserialize");
  assert_eq!(restored, entry);
}

#[test]
fn block_and_unblock_round_trip() {
  with_runtime(|| {
    let state = BlacklistState::new();
    let target = user("alice");
    assert!(!state.is_blocked(&target));
    state.block(target.clone(), "Alice".into());
    assert!(state.is_blocked(&target));
    state.unblock(&target);
    assert!(!state.is_blocked(&target));
  });
}

#[test]
fn double_block_is_idempotent() {
  with_runtime(|| {
    let state = BlacklistState::new();
    let target = user("bob");
    state.block(target.clone(), "Bob".into());
    state.block(target.clone(), "Bob renamed".into());
    assert_eq!(state.count(), 1);
    let entries = state.list();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].display_name, "Bob");
  });
}

#[test]
fn has_pending_auto_decline_default_false() {
  with_runtime(|| {
    let state = BlacklistState::new();
    let target = user("carol");
    assert!(!state.has_pending_auto_decline(&target));
  });
}

#[test]
fn cancel_all_auto_decline_clears_table() {
  with_runtime(|| {
    let state = BlacklistState::new();
    // We cannot register a real `TimeoutHandle` outside a browser,
    // but we can verify that `cancel_all_auto_decline` is safe to
    // call when the table is empty.
    state.cancel_all_auto_decline();
    let target = user("dave");
    assert!(!state.has_pending_auto_decline(&target));
  });
}

#[test]
fn forget_auto_decline_for_unknown_user_is_noop() {
  with_runtime(|| {
    let state = BlacklistState::new();
    let target = user("eve");
    state.forget_auto_decline(&target);
    assert!(!state.has_pending_auto_decline(&target));
  });
}
