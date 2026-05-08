//! Reactive settings state and localStorage persistence.
//!
//! [`SettingsState`] wraps an [`RwSignal<UserSettings>`] and provides
//! a mutation API that auto-persists every change to `localStorage`.
//! Components consume it via Leptos context.

use leptos::prelude::*;

use super::types::UserSettings;
use crate::utils;

/// localStorage key used to persist the entire [`UserSettings`] blob.
///
/// Namespaced with the `settings_` prefix required by Req 13
/// Technical Implementation Constraints #1. A legacy `user_settings`
/// key (pre-migration) is still read-through once on startup so
/// existing deployments keep their preferences.
pub const STORAGE_KEY: &str = "settings_user";

/// Legacy storage key, read as a one-time fallback during migration.
pub const LEGACY_STORAGE_KEY: &str = "user_settings";

/// Reactive settings store exposed via Leptos context.
#[derive(Debug, Clone, Copy)]
pub struct SettingsState {
  /// Underlying signal. Components mutate it via
  /// [`SettingsState::update`] so persistence stays in lock-step.
  inner: RwSignal<UserSettings>,
  /// Monotonically increasing counter that bumps every time the
  /// settings store is mutated. The settings UI reads this signal to
  /// drive the "Saved" feedback indicator (Req 13.6.3) without having
  /// to wire a per-row ack signal everywhere.
  saved_at: RwSignal<u64>,
}

impl SettingsState {
  /// Create a fresh store, loading any previously persisted snapshot.
  #[must_use]
  pub fn new() -> Self {
    let initial = load_from_storage().unwrap_or_default().sanitised();
    Self {
      inner: RwSignal::new(initial),
      saved_at: RwSignal::new(0),
    }
  }

  /// Subscribe to the current snapshot (reactive).
  pub fn get(self) -> UserSettings {
    self.inner.get()
  }

  /// Raw handle for components that want to derive sub-signals with
  /// `with` / `update`.
  #[must_use]
  pub fn signal(self) -> RwSignal<UserSettings> {
    self.inner
  }

  /// Reactive "save tick" — increments every successful
  /// [`SettingsState::update`] call. UI can subscribe to this signal
  /// to render a transient "Saved" indicator.
  #[must_use]
  pub fn saved_tick(self) -> RwSignal<u64> {
    self.saved_at
  }

  /// Apply a mutation closure and persist the resulting snapshot.
  pub fn update<F>(self, mutate: F)
  where
    F: FnOnce(&mut UserSettings),
  {
    self.inner.update(|settings| {
      mutate(settings);
      *settings = settings.clone().sanitised();
    });
    let snapshot = self.inner.get_untracked();
    persist_to_storage(&snapshot);
    self.saved_at.update(|n| *n = n.wrapping_add(1));
  }

  /// Explicitly bump the `saved_at` tick without mutating the
  /// underlying snapshot. Used by adjacent surfaces (theme / locale)
  /// that own their own persistence path but still participate in
  /// the unified "Saved" feedback indicator (Req 13.6.3).
  pub fn bump_saved(self) {
    self.saved_at.update(|n| *n = n.wrapping_add(1));
  }
}

impl Default for SettingsState {
  fn default() -> Self {
    Self::new()
  }
}

/// Install the settings store as a Leptos context value.
pub fn provide_settings_state() -> SettingsState {
  let state = SettingsState::new();
  provide_context(state);
  state
}

/// Retrieve the settings store from context.
///
/// # Panics
/// Panics if [`provide_settings_state`] has not been called.
#[must_use]
pub fn use_settings_state() -> SettingsState {
  expect_context::<SettingsState>()
}

// ── Persistence helpers ────────────────────────────────────────────────

pub(super) fn load_from_storage() -> Option<UserSettings> {
  // Prefer the canonical `settings_user` key. When it is missing, try
  // the legacy `user_settings` blob so upgrades from the pre-migration
  // build preserve the user's preferences. The legacy key is deleted
  // opportunistically once a successful migration has been observed.
  if let Some(raw) = utils::load_from_local_storage(STORAGE_KEY)
    && let Ok(parsed) = serde_json::from_str::<UserSettings>(&raw)
  {
    return Some(parsed);
  }
  let legacy = utils::load_from_local_storage(LEGACY_STORAGE_KEY)?;
  let parsed = serde_json::from_str::<UserSettings>(&legacy)
    .ok()?
    .sanitised();
  // Migrate: write under the new key and drop the old one.
  if let Ok(json) = serde_json::to_string(&parsed) {
    utils::save_to_local_storage(STORAGE_KEY, &json);
  }
  utils::remove_from_local_storage(LEGACY_STORAGE_KEY);
  Some(parsed)
}

/// Load the current settings snapshot directly from `localStorage`,
/// bypassing the Leptos reactive context.
///
/// This is intended for callers that run outside a Leptos owner — for
/// example browser event callbacks, `setInterval` handlers, or
/// background async tasks — where [`use_settings_state`] is not
/// available. Returns the default settings when no snapshot has been
/// persisted yet.
#[must_use]
pub fn load_snapshot() -> UserSettings {
  load_from_storage().unwrap_or_default().sanitised()
}

pub fn persist_to_storage(settings: &UserSettings) {
  if let Ok(json) = serde_json::to_string(settings) {
    utils::save_to_local_storage(STORAGE_KEY, &json);
  }
}
