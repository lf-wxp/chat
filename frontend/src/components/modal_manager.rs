//! Modal manager component.
//!
//! Hosts global modals that can be triggered from anywhere in the app
//! (incoming room invites, create-room, password-prompt, …). Components
//! inside this manager render lazily based on `app_state` signals and
//! the shared `GlobalRoomModalState` context.

use leptos::prelude::*;
use leptos_i18n::t_string;

use crate::components::room::global_modal_context::GlobalRoomModalState;
use crate::components::room::{
  CreateRoomModal, CreateRoomRequest, IncomingRoomInviteModal, PasswordPromptModal,
};
use crate::error_handler::use_error_toast_manager;
use crate::i18n;
use crate::signaling::use_signaling_client;

/// Modal manager component.
#[component]
pub fn ModalManager() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let signaling = use_signaling_client();
  let toast = use_error_toast_manager();
  let modal_state = GlobalRoomModalState::use_global();

  let signaling_for_create = signaling.clone();
  let toast_for_create = toast;
  let on_create_submit = Callback::new(move |req: CreateRoomRequest| {
    if let Err(e) = signaling_for_create.send_create_room(
      req.name.clone(),
      req.description.clone(),
      req.room_type,
      req.password.clone(),
    ) {
      web_sys::console::warn_1(&format!("[room] Failed to create room: {e}").into());
      toast_for_create.show_error_message_with_key(
        "ROM109",
        "error.rom109",
        t_string!(i18n, error.rom109),
      );
    }
  });

  let password_title = Signal::derive(move || {
    modal_state
      .password_target
      .with(|t| t.as_ref().map(|r| r.name.clone()))
      .unwrap_or_default()
  });

  let signaling_for_join = signaling.clone();
  let toast_for_join = toast;
  let on_password_submit = Callback::new(move |pwd: String| {
    if let Some(room) = modal_state.password_target.get()
      && let Err(e) = signaling_for_join.send_join_room(room.room_id.clone(), Some(pwd))
    {
      web_sys::console::warn_1(&format!("[room] Failed to join room: {e}").into());
      toast_for_join.show_error_message_with_key(
        "ROM109",
        "error.rom109",
        t_string!(i18n, error.rom109),
      );
    }
    modal_state.password_target.set(None);
  });

  let on_password_cancel = Callback::new(move |()| {
    modal_state.password_target.set(None);
  });

  view! {
    <div id="modal-root">
      <IncomingRoomInviteModal />
      <CreateRoomModal open=modal_state.create_open on_submit=on_create_submit />
      <Show when=move || modal_state.password_target.with(Option::is_some)>
        <PasswordPromptModal
          title=password_title
          on_submit=on_password_submit
          on_cancel=on_password_cancel
        />
      </Show>
    </div>
  }
}
