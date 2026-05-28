//! Global room modal context (Req 4.1 / 4.2).
//!
//! Provides signals that allow any component in the tree to trigger
//! the create-room, password-prompt, invite-member and user-info
//! modals while they are rendered at the app root level (inside
//! `ModalManager`). This avoids the modal being clipped by an
//! ancestor's containing-block-creating property — most notably
//! `backdrop-filter` on `.sidebar`, which silently traps any
//! sidebar-hosted `position: fixed` overlay inside the sidebar's
//! 16rem-wide column.

use leptos::prelude::*;
use message::UserId;
use message::types::RoomInfo;

/// Context shared across the app to open/close the global modals.
///
/// All overlay modals are rendered at the app root inside
/// `ModalManager` so that no ancestor (sidebar / drawer / `.app`) can
/// become their containing block. Triggers anywhere in the tree
/// mutate the signals below to open/close each modal — they never
/// render the modal node themselves.
#[derive(Clone, Copy)]
pub struct GlobalRoomModalState {
  /// Whether the create-room modal is open.
  pub create_open: RwSignal<bool>,
  /// The room that needs a password prompt (Some = open, None = closed).
  pub password_target: RwSignal<Option<RoomInfo>>,
  /// The room whose owner is sending invites (Some = open, None = closed).
  pub invite_target: RwSignal<Option<RoomInfo>>,
  /// The user whose info card is being displayed (Some = open,
  /// None = closed). Hosted globally so the card escapes the sidebar's
  /// `backdrop-filter` containing block.
  pub user_info_target: RwSignal<Option<UserId>>,
}

impl GlobalRoomModalState {
  /// Create and `provide_context` a new instance. Must be called from
  /// a component above both the trigger site and the renderer.
  pub fn provide() -> Self {
    let state = Self {
      create_open: RwSignal::new(false),
      password_target: RwSignal::new(None),
      invite_target: RwSignal::new(None),
      user_info_target: RwSignal::new(None),
    };
    provide_context(state);
    state
  }

  /// Retrieve the context previously provided by [`Self::provide`].
  pub fn use_global() -> Self {
    use_context::<Self>().expect("GlobalRoomModalState must be provided at app root")
  }
}
