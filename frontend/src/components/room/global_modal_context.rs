//! Global room modal context (Req 4.1 / 4.2 UI fix).
//!
//! Provides signals that allow any component in the tree to trigger
//! the create-room or password-prompt modals while they are rendered
//! at the app root level (inside `ModalManager`). This avoids the
//! modal being clipped by an ancestor's `overflow: hidden`.

use leptos::prelude::*;
use message::types::RoomInfo;

/// Context shared across the app to open/close the global create-room
/// and password-prompt modals.
#[derive(Clone, Copy)]
pub struct GlobalRoomModalState {
  /// Whether the create-room modal is open.
  pub create_open: RwSignal<bool>,
  /// The room that needs a password prompt (Some = open, None = closed).
  pub password_target: RwSignal<Option<RoomInfo>>,
}

impl GlobalRoomModalState {
  /// Create and `provide_context` a new instance. Must be called from
  /// a component above both the trigger site and the renderer.
  pub fn provide() -> Self {
    let state = Self {
      create_open: RwSignal::new(false),
      password_target: RwSignal::new(None),
    };
    provide_context(state);
    state
  }

  /// Retrieve the context previously provided by [`Self::provide`].
  pub fn use_global() -> Self {
    use_context::<Self>().expect("GlobalRoomModalState must be provided at app root")
  }
}
