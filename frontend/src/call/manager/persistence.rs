//! LocalStorage-backed refresh recovery for [`super::CallManager`]
//! (P2-New-1 split — Req 10.5).
//!
//! Stores the currently-active call (both `Inviting` and `Active`
//! phases) in `localStorage` so a page refresh can offer to rejoin.
//! The `StoredCall` struct is versioned via `#[serde(default)]` so
//! older payloads that pre-date later additions (`screen_sharing`,
//! `phase`) decode without losing data.

use leptos::prelude::*;
use message::RoomId;
use message::types::MediaType;
use serde::{Deserialize, Serialize};

use super::{CallManager, CallPhase, CallState, PersistedCallState};

/// localStorage key for persisted call state (Req 10.5).
pub(super) const STORAGE_KEY: &str = "active_call";

/// Serialisation wrapper stored in localStorage.
///
/// Kept separate from [`PersistedCallState`] so future extensions
/// (e.g. media settings) can be added without breaking the recovery
/// format via `#[serde(default)]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct StoredCall {
  pub(super) room_id: RoomId,
  pub(super) media_type: MediaType,
  pub(super) started_at_ms: i64,
  #[serde(default)]
  pub(super) screen_sharing: bool,
  #[serde(default)]
  pub(super) phase: CallPhase,
}

impl From<&StoredCall> for PersistedCallState {
  fn from(value: &StoredCall) -> Self {
    Self {
      room_id: value.room_id.clone(),
      media_type: value.media_type,
      started_at_ms: value.started_at_ms,
      screen_sharing: value.screen_sharing,
      phase: value.phase,
    }
  }
}

impl CallManager {
  /// Persist the current call state to localStorage so a page refresh
  /// can offer to rejoin (Req 10.5). No-op outside `Inviting`/`Active`.
  pub(super) fn persist(&self) {
    let current = self.signals.call_state.get_untracked();
    let (room_id, media_type, started_at_ms, phase) = match current {
      CallState::Inviting {
        room_id,
        media_type,
        started_at_ms,
      } => (room_id, media_type, started_at_ms, CallPhase::Inviting),
      CallState::Active {
        room_id,
        media_type,
        started_at_ms,
      } => (room_id, media_type, started_at_ms, CallPhase::Active),
      _ => return,
    };
    let stored = StoredCall {
      room_id,
      media_type,
      started_at_ms,
      screen_sharing: self.signals.local_media.get_untracked().screen_sharing,
      phase,
    };
    if let Ok(json) = serde_json::to_string(&stored) {
      crate::utils::save_to_local_storage(STORAGE_KEY, &json);
    }
  }

  /// Clear the localStorage entry so subsequent refreshes do not offer
  /// a recovery prompt for a call that has already ended.
  pub(super) fn clear_persist(&self) {
    crate::utils::remove_from_local_storage(STORAGE_KEY);
  }
}

/// Read the persisted call state from localStorage, returning `None`
/// when nothing is stored or the stored payload is malformed.
#[must_use]
pub fn load_persisted() -> Option<PersistedCallState> {
  let raw = crate::utils::load_from_local_storage(STORAGE_KEY)?;
  if raw.is_empty() {
    return None;
  }
  serde_json::from_str::<StoredCall>(&raw)
    .ok()
    .as_ref()
    .map(PersistedCallState::from)
}

#[cfg(test)]
mod tests;
