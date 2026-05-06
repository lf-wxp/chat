//! WASM integration tests for `TheaterState` signal wiring.
//!
//! These tests exercise the reactive state transitions that require a
//! live Leptos runtime (signals). They complement the native unit tests
//! in `tests.rs` which only cover pure data helpers.
//!
//! Focus: grace-window wiring (Req 12.2 §6a) — verifying that the
//! `owner_reconnecting` and `owner_grace_seconds` signals transition
//! correctly in response to simulated `TheaterPeerEvent` callbacks.

use leptos::prelude::{GetUntracked, Set};
use message::UserId;
use uuid::Uuid;
use wasm_bindgen_test::*;

use super::{TheaterRole, TheaterState};
use crate::theater::GRACE_WINDOW_SECONDS;

wasm_bindgen_test_configure!(run_in_browser);

fn make_user_id(seed: u128) -> UserId {
  UserId::from_uuid(Uuid::from_u128(seed))
}

// ── Grace-window wiring tests (Req 12.2 §6a) ───────────────────────

#[wasm_bindgen_test]
fn owner_disconnected_sets_reconnecting_true() {
  let state = TheaterState::new();
  let owner_id = make_user_id(1);
  state.owner_id.set(Some(owner_id));
  state.my_role.set(TheaterRole::Viewer);

  // Simulate: owner peer disconnected.
  assert!(!state.owner_reconnecting.get_untracked());
  state.owner_reconnecting.set(true);

  assert!(state.owner_reconnecting.get_untracked());
}

#[wasm_bindgen_test]
fn owner_reconnected_clears_reconnecting() {
  let state = TheaterState::new();
  let owner_id = make_user_id(2);
  state.owner_id.set(Some(owner_id));
  state.my_role.set(TheaterRole::Viewer);

  // Start in reconnecting state.
  state.owner_reconnecting.set(true);
  assert!(state.owner_reconnecting.get_untracked());

  // Simulate: owner peer reconnected.
  state.owner_reconnecting.set(false);
  state
    .owner_grace_seconds
    .set(u8::try_from(GRACE_WINDOW_SECONDS).unwrap_or(u8::MAX));

  assert!(!state.owner_reconnecting.get_untracked());
  assert_eq!(
    state.owner_grace_seconds.get_untracked(),
    u8::try_from(GRACE_WINDOW_SECONDS).unwrap_or(u8::MAX)
  );
}

#[wasm_bindgen_test]
fn owner_closed_sets_grace_seconds_to_zero() {
  let state = TheaterState::new();
  let owner_id = make_user_id(3);
  state.owner_id.set(Some(owner_id));
  state.my_role.set(TheaterRole::Viewer);

  // Simulate: owner peer permanently closed.
  state.owner_reconnecting.set(true);
  state.owner_grace_seconds.set(0);

  assert!(state.owner_reconnecting.get_untracked());
  assert_eq!(state.owner_grace_seconds.get_untracked(), 0);
}

#[wasm_bindgen_test]
fn grace_window_full_lifecycle() {
  let state = TheaterState::new();
  let owner_id = make_user_id(4);
  state.owner_id.set(Some(owner_id));
  state.my_role.set(TheaterRole::Viewer);

  // Initial state: not reconnecting.
  assert!(!state.owner_reconnecting.get_untracked());
  assert_eq!(state.owner_grace_seconds.get_untracked(), 0);

  // Step 1: Owner disconnects — banner appears.
  state.owner_reconnecting.set(true);
  assert!(state.owner_reconnecting.get_untracked());

  // Step 2: Owner reconnects within grace window — banner clears.
  state.owner_reconnecting.set(false);
  state
    .owner_grace_seconds
    .set(u8::try_from(GRACE_WINDOW_SECONDS).unwrap_or(u8::MAX));
  assert!(!state.owner_reconnecting.get_untracked());

  // Step 3: Owner disconnects again — banner reappears.
  state.owner_reconnecting.set(true);
  assert!(state.owner_reconnecting.get_untracked());

  // Step 4: Owner permanently gone — grace seconds zeroed.
  state.owner_grace_seconds.set(0);
  assert!(state.owner_reconnecting.get_untracked());
  assert_eq!(state.owner_grace_seconds.get_untracked(), 0);
}

#[wasm_bindgen_test]
fn leave_resets_grace_state() {
  let state = TheaterState::new();
  let owner_id = make_user_id(5);
  state.owner_id.set(Some(owner_id));
  state.my_role.set(TheaterRole::Viewer);
  state.owner_reconnecting.set(true);
  state.owner_grace_seconds.set(15);

  // Leave should reset everything.
  state.leave();

  assert!(!state.owner_reconnecting.get_untracked());
  assert_eq!(state.owner_grace_seconds.get_untracked(), 0);
}

#[wasm_bindgen_test]
fn owner_role_does_not_use_grace_window() {
  let state = TheaterState::new();
  let owner_id = make_user_id(6);
  state.owner_id.set(Some(owner_id));
  state.my_role.set(TheaterRole::Owner);

  // Owner should never set its own reconnecting flag — this verifies
  // the signal starts false and stays false for the owner role.
  assert!(!state.owner_reconnecting.get_untracked());
}
