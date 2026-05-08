//! User settings module (Task 23 / Req 13).
//!
//! Manages all user-configurable preferences outside of the primary
//! appearance + privacy surface already owned by `AppState`:
//!
//! * Audio/video defaults (device ids, speaker volume, video quality)
//! * Typography (font scale)
//! * Privacy toggles (online status visibility, read receipts)
//! * Notifications (message / call / do-not-disturb window)
//! * Data management policy (retention window)
//!
//! Settings are held in a single [`UserSettings`] record exposed as a
//! [`RwSignal`] via Leptos context. Mutations are persisted to
//! `localStorage` immediately so a page refresh restores the exact
//! preferences without any intermediate blank state.
//!
//! ## Module layout
//!
//! * [`types`] — pure data types (`FontScale`, `VideoQualityPref`,
//!   `DndWindow`, `UserSettings`).
//! * [`state`] — reactive `SettingsState` + localStorage persistence.
//! * [`export`] — `ExportPayload` and JSON / HTML rendering.

mod export;
mod state;
mod types;

// Re-export the public API so existing callers remain unchanged.
pub use export::ExportPayload;
// Re-export for test access (via `use super::*` in tests.rs).
#[cfg(test)]
pub(crate) use export::html_escape;
pub use state::{
  LEGACY_STORAGE_KEY, STORAGE_KEY, SettingsState, load_snapshot, persist_to_storage,
  provide_settings_state, use_settings_state,
};
pub use types::{
  BACKGROUND_BLUR_MAX_PX, BACKGROUND_OVERLAY_ALPHA_MAX, BackgroundMode, BackgroundSettings,
  BackgroundVariantData, BackgroundVariantView, DndWindow, FontScale, GradientKind, GradientSpec,
  GradientStop, UserSettings, VOLUME_MAX, VideoQualityPref,
};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod wasm_tests;
