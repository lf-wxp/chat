//! Client-side blacklist (block list) management.
//!
//! Implements **Req 9.2** (Blacklist Functionality):
//!
//! - Blocked user ids and their block timestamps are stored in
//!   `localStorage` only — never synchronised with the server, so the
//!   block action remains private to the local browser.
//! - When a user is added to the blacklist, the active WebRTC peer
//!   connection (if any) is closed silently (no signaling beyond the
//!   normal `PeerClosed`).
//! - Inbound connection invitations from blocked users are auto-declined
//!   after a randomised 30–60 s delay so the blocked user observes
//!   normal-looking timeout behaviour (Req 9.17).
//!
//! The blacklist data lives in a Leptos signal so that all UI surfaces
//! (online users panel, user info card, blacklist management panel)
//! refresh reactively when the set changes.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use leptos::prelude::*;
use message::UserId;
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use crate::utils;
use crate::utils::TimeoutHandle;

/// localStorage key used to persist the blacklist between page reloads.
pub const STORAGE_KEY: &str = "blacklist";

/// Minimum auto-decline delay (Req 9.17 — 30 s lower bound).
pub const AUTO_DECLINE_MIN_MS: u32 = 30_000;

/// Maximum auto-decline delay (Req 9.17 — 60 s upper bound).
pub const AUTO_DECLINE_MAX_MS: u32 = 60_000;

/// One blacklist entry. Stores the block timestamp so the management
/// panel can render an "added on" column.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlacklistEntry {
  /// Blocked user id.
  pub user_id: UserId,
  /// Snapshot of the user's display name at the time of blocking, used
  /// when rendering the management panel even if the user is currently
  /// offline (and therefore absent from `online_users`).
  pub display_name: String,
  /// Unix-ms timestamp of when the block was created.
  pub blocked_at_ms: i64,
}

/// Reactive blacklist state.
///
/// Cloning is cheap — internal state is stored in a single `RwSignal`
/// (which is `Copy`) plus an `Rc<RefCell<...>>` for the non-reactive
/// pending auto-decline timer table.
#[derive(Debug, Clone)]
pub struct BlacklistState {
  /// Map of blocked user id → entry. A `HashMap` keeps lookup O(1) on
  /// the hot path (`is_blocked`) which is invoked for every inbound
  /// invitation and for every row of the online users panel.
  entries: RwSignal<HashMap<UserId, BlacklistEntry>>,
  /// Per-blocked-user pending auto-decline timer (Req 9.17). Stored
  /// here so [`Self::unblock`] / [`Self::shutdown`] can cancel
  /// pending fires, and so a duplicate invite from the same blocked
  /// user reuses the existing timer rather than allocating a new one.
  ///
  /// Wrapped in `Rc<RefCell<...>>` because `TimeoutHandle` is not
  /// `Copy` and we need cheap clones of the entire `BlacklistState`
  /// for context propagation.
  pending_auto_decline: Rc<RefCell<HashMap<UserId, TimeoutHandle>>>,
}

crate::wasm_send_sync!(BlacklistState);

impl BlacklistState {
  /// Create a fresh blacklist state and hydrate it from `localStorage`.
  ///
  /// On native (non-WASM) builds the localStorage path is skipped so
  /// unit tests can construct a state without reaching into
  /// `web_sys::window`, which panics under wasm-bindgen on non-WASM
  /// targets.
  #[must_use]
  pub fn new() -> Self {
    #[cfg(target_arch = "wasm32")]
    let entries = load_from_storage();
    #[cfg(not(target_arch = "wasm32"))]
    let entries = HashMap::new();
    Self {
      entries: RwSignal::new(entries),
      pending_auto_decline: Rc::new(RefCell::new(HashMap::new())),
    }
  }

  /// Returns `true` when the user is currently blocked.
  #[must_use]
  pub fn is_blocked(&self, user_id: &UserId) -> bool {
    self.entries.with(|map| map.contains_key(user_id))
  }

  /// Untracked variant of [`Self::is_blocked`] for use outside the
  /// Leptos reactive owner (e.g. WebSocket message callbacks).
  #[must_use]
  pub fn is_blocked_untracked(&self, user_id: &UserId) -> bool {
    self.entries.with_untracked(|map| map.contains_key(user_id))
  }

  /// Snapshot of every entry sorted by most-recent-first.
  #[must_use]
  pub fn list(&self) -> Vec<BlacklistEntry> {
    let mut out: Vec<BlacklistEntry> = self.entries.with(|map| map.values().cloned().collect());
    out.sort_by_key(|e| std::cmp::Reverse(e.blocked_at_ms));
    out
  }

  /// Add `user_id` to the blacklist. Has no effect if the user is
  /// already blocked. Persists to localStorage on success.
  pub fn block(&self, user_id: UserId, display_name: String) {
    let entry = BlacklistEntry {
      user_id: user_id.clone(),
      display_name,
      blocked_at_ms: chrono::Utc::now().timestamp_millis(),
    };
    let changed = self.entries.try_update(|map| {
      if let std::collections::hash_map::Entry::Vacant(slot) = map.entry(user_id) {
        slot.insert(entry);
        true
      } else {
        false
      }
    });
    if matches!(changed, Some(true)) {
      self.persist();
    }
  }

  /// Remove `user_id` from the blacklist. Has no effect if the user is
  /// not currently blocked. Cancels any pending auto-decline timer for
  /// that user so a subsequent (post-unblock) invite is no longer
  /// silently rejected (Req 9.19). Persists to localStorage on success.
  pub fn unblock(&self, user_id: &UserId) {
    let changed = self.entries.try_update(|map| map.remove(user_id).is_some());
    if matches!(changed, Some(true)) {
      self.cancel_auto_decline(user_id);
      self.persist();
    }
  }

  /// Number of currently blocked users.
  #[must_use]
  pub fn count(&self) -> usize {
    self.entries.with(|map| map.len())
  }

  /// Reactive accessor for the underlying signal — used by UI lists
  /// that need to re-render on any change.
  #[must_use]
  pub fn signal(&self) -> RwSignal<HashMap<UserId, BlacklistEntry>> {
    self.entries
  }

  /// Register a pending auto-decline timer for an inbound invite from
  /// a blocked user (Req 9.17). Returns `true` when the timer was
  /// registered and `false` when an existing timer for the same
  /// inviter is reused (deduplication).
  ///
  /// The caller is responsible for arming the actual `setTimeout`;
  /// this method only owns the resulting handle so it can be cancelled
  /// on `unblock` / `shutdown`.
  pub fn register_auto_decline(&self, user_id: UserId, handle: TimeoutHandle) -> bool {
    let mut table = self.pending_auto_decline.borrow_mut();
    if table.contains_key(&user_id) {
      // Deduplicate: cancel the new handle, keep the existing one.
      handle.cancel();
      return false;
    }
    table.insert(user_id, handle);
    true
  }

  /// Returns `true` when an auto-decline timer is currently pending
  /// for `user_id`. Used by the signaling layer to skip arming a
  /// duplicate timer for back-to-back invites from the same blocked
  /// inviter.
  #[must_use]
  pub fn has_pending_auto_decline(&self, user_id: &UserId) -> bool {
    self.pending_auto_decline.borrow().contains_key(user_id)
  }

  /// Drop the pending timer registration for `user_id` once it has
  /// fired (so the slot can be reused for a future invite from the
  /// same inviter). Does **not** cancel the timer — it has already
  /// fired by the time this method runs.
  pub fn forget_auto_decline(&self, user_id: &UserId) {
    self.pending_auto_decline.borrow_mut().remove(user_id);
  }

  /// Cancel a pending auto-decline timer for `user_id` (e.g. when the
  /// user is unblocked before the 30-60 s window elapses).
  pub fn cancel_auto_decline(&self, user_id: &UserId) {
    if let Some(handle) = self.pending_auto_decline.borrow_mut().remove(user_id) {
      handle.cancel();
    }
  }

  /// Cancel every pending auto-decline timer (e.g. on logout).
  pub fn cancel_all_auto_decline(&self) {
    let drained: Vec<TimeoutHandle> = self
      .pending_auto_decline
      .borrow_mut()
      .drain()
      .map(|(_, h)| h)
      .collect();
    for handle in drained {
      handle.cancel();
    }
  }

  fn persist(&self) {
    // Skip the localStorage round-trip on native (non-WASM) test
    // builds — `web_sys::window` is not callable there and would
    // panic. Persistence is only meaningful in the browser anyway.
    #[cfg(target_arch = "wasm32")]
    {
      let snapshot: Vec<BlacklistEntry> = self
        .entries
        .with_untracked(|map| map.values().cloned().collect());
      if let Ok(json) = serde_json::to_string(&snapshot) {
        utils::save_to_local_storage(STORAGE_KEY, &json);
      }
    }
  }
}

impl Default for BlacklistState {
  fn default() -> Self {
    Self::new()
  }
}

/// Compute a randomised auto-decline delay within the `[30s, 60s]`
/// range (Req 9.17). Pure function so it can be unit-tested without a
/// browser; non-test callers should use [`random_auto_decline_delay_ms`]
/// which seeds from `js_sys::Math::random` on WASM.
#[must_use]
pub fn auto_decline_delay_from_seed(seed: f64) -> u32 {
  let clamped = seed.clamp(0.0, 1.0);
  let span = AUTO_DECLINE_MAX_MS - AUTO_DECLINE_MIN_MS;
  // `as u32` after multiplication is safe because `span < u32::MAX` and
  // `clamped <= 1.0`, so the product fits in `u32`.
  AUTO_DECLINE_MIN_MS + (f64::from(span) * clamped) as u32
}

/// Sample a random auto-decline delay. Uses `js_sys::Math::random` on
/// WASM and a deterministic mid-range value on native test runs.
#[must_use]
pub fn random_auto_decline_delay_ms() -> u32 {
  #[cfg(target_arch = "wasm32")]
  {
    auto_decline_delay_from_seed(js_sys::Math::random())
  }
  #[cfg(not(target_arch = "wasm32"))]
  {
    auto_decline_delay_from_seed(0.5)
  }
}

/// Provide a fresh `BlacklistState` to the Leptos context. Returns the
/// state so the caller can keep a non-context handle if needed.
pub fn provide_blacklist_state() -> BlacklistState {
  let state = BlacklistState::new();
  provide_context(state.clone());
  state
}

/// Retrieve the current `BlacklistState` from Leptos context.
///
/// # Panics
/// Panics if `provide_blacklist_state` has not been called.
#[must_use]
pub fn use_blacklist_state() -> BlacklistState {
  expect_context::<BlacklistState>()
}

/// Best-effort variant of [`use_blacklist_state`] safe to call from
/// non-reactive callbacks; returns `None` outside the Leptos owner.
#[must_use]
pub fn try_use_blacklist_state() -> Option<BlacklistState> {
  use_context::<BlacklistState>()
}

#[cfg(target_arch = "wasm32")]
fn load_from_storage() -> HashMap<UserId, BlacklistEntry> {
  let Some(raw) = utils::load_from_local_storage(STORAGE_KEY) else {
    return HashMap::new();
  };
  match serde_json::from_str::<Vec<BlacklistEntry>>(&raw) {
    Ok(entries) => entries
      .into_iter()
      .map(|e| (e.user_id.clone(), e))
      .collect(),
    Err(_) => HashMap::new(),
  }
}

#[cfg(test)]
mod tests;
