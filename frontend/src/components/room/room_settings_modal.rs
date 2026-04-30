//! Room settings modal (Owner only — Req 4.5 / 4.5a / 4.5b).
//!
//! Lets the owner edit the room's name and description, change the
//! password (with two-entry confirmation per Req 4.5a) or clear it
//! entirely (Req 4.5b). All updates are dispatched through the
//! signaling client and the modal closes itself once the request
//! has been queued.

use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use message::error::validation::{
  validate_room_description, validate_room_name, validate_room_password,
};
use message::types::RoomInfo;

use crate::components::room::confirm_dialog::{ConfirmDialog, ConfirmTone};
use crate::components::room::modal_wrapper::{ModalSize, ModalWrapper};
use crate::components::room::password_prompt_modal::{PasswordPromptModal, PasswordPromptMode};
use crate::error_handler::use_error_toast_manager;
use crate::i18n;
use crate::signaling::use_signaling_client;
use icondata as i;
use leptos_icons::Icon;

/// Room settings modal.
#[component]
pub fn RoomSettingsModal(
  /// Reactive flag driving visibility. The modal closes itself by
  /// setting this to `false` after submitting / cancelling.
  open: RwSignal<bool>,
  /// The room being edited. Required so the modal can prefill the
  /// name and description fields.
  #[prop(into)]
  room: Signal<RoomInfo>,
) -> impl IntoView {
  let i18n = i18n::use_i18n();
  let signaling = use_signaling_client();
  let toast = use_error_toast_manager();

  // The signaling client is `Clone` but not `Copy`, so we wrap it in
  // a `StoredValue` to satisfy `Fn` requirements imposed by Leptos
  // event handlers / view closures.
  let signaling_store = StoredValue::new(signaling);

  let name = RwSignal::new(String::new());
  let description = RwSignal::new(String::new());
  let error = RwSignal::new(Option::<String>::None);
  let password_open = RwSignal::new(false);
  let confirm_clear_open = RwSignal::new(false);

  // Reset form whenever the modal is reopened or the source room
  // changes (the owner may switch between rooms while the modal is
  // mounted).
  Effect::new(move |_| {
    if open.get() {
      let r = room.get();
      name.set(r.name);
      description.set(r.description);
      error.set(None);
    }
  });

  let handle_submit = move |ev: leptos::ev::SubmitEvent| {
    ev.prevent_default();
    let new_name = name.get();
    let new_description = description.get();

    if let Err(e) = validate_room_name(&new_name) {
      error.set(Some(e.message));
      return;
    }
    if !new_description.is_empty()
      && let Err(e) = validate_room_description(&new_description)
    {
      error.set(Some(e.message));
      return;
    }
    error.set(None);

    let room_id = room.with(|r| r.room_id.clone());
    let result = signaling_store
      .with_value(|s| s.send_update_room_info(room_id, new_name, new_description.clone()));
    if let Err(e) = result {
      web_sys::console::warn_1(&format!("[room] Failed to update room info: {e}").into());
      toast.show_error_message_with_key("ROM112", "error.rom112", t_string!(i18n, error.rom112));
      return;
    }
    toast.show_info_message_with_key(
      "ROM2000",
      "room.settings_info_updated",
      t_string!(i18n, room.settings_info_updated),
    );
    open.set(false);
  };

  let on_password_submit = Callback::new(move |pwd: String| {
    let room_id = room.with(|r| r.room_id.clone());
    if let Err(e) = validate_room_password(&pwd) {
      toast.show_error_message_with_key("ROM2102", "error.rom2102", &e.message);
      return;
    }
    let result = signaling_store.with_value(|s| s.send_update_room_password(room_id, Some(pwd)));
    if let Err(e) = result {
      web_sys::console::warn_1(&format!("[room] Failed to update password: {e}").into());
      toast.show_error_message_with_key("ROM112", "error.rom112", t_string!(i18n, error.rom112));
      return;
    }
    toast.show_info_message_with_key(
      "ROM2100",
      "room.settings_password_updated",
      t_string!(i18n, room.settings_password_updated),
    );
    password_open.set(false);
  });
  let on_password_cancel = Callback::new(move |()| password_open.set(false));

  let on_clear_confirm = Callback::new(move |()| {
    let room_id = room.with(|r| r.room_id.clone());
    let result = signaling_store.with_value(|s| s.send_update_room_password(room_id, None));
    if let Err(e) = result {
      web_sys::console::warn_1(&format!("[room] Failed to clear password: {e}").into());
      toast.show_error_message_with_key("ROM112", "error.rom112", t_string!(i18n, error.rom112));
      confirm_clear_open.set(false);
      return;
    }
    toast.show_info_message_with_key(
      "ROM2100",
      "room.settings_password_cleared",
      t_string!(i18n, room.settings_password_cleared),
    );
    confirm_clear_open.set(false);
  });
  let on_clear_cancel = Callback::new(move |()| confirm_clear_open.set(false));

  let is_protected = Memo::new(move |_| room.with(message::types::RoomInfo::is_password_protected));

  view! {
    <Show when=move || open.get()>
      <ModalWrapper
        on_close=Callback::new(move |()| open.set(false))
        size=ModalSize::Medium
        class="room-settings-modal"
        labelled_by="room-settings-title"
        testid="room-settings-modal"
      >
        <form
          class="room-settings-modal__form"
          on:submit=handle_submit
        >
          <header class="modal-header">
            <h2 id="room-settings-title" class="modal-title">{t!(i18n, room.settings)}</h2>
            <button
              type="button"
              class="modal-close"
              aria-label=move || t_string!(i18n, common.close)
              on:click=move |_| open.set(false)
            ><Icon icon=i::LuX /></button>
          </header>

          <div class="modal-body room-settings-modal__body">
            <label class="room-settings-modal__label" for="room-settings-name">
              {t!(i18n, room.room_name)}
            </label>
            <input
              id="room-settings-name"
              class="input"
              type="text"
              maxlength="100"
              required
              prop:value=move || name.get()
              on:input=move |ev| name.set(event_target_value(&ev))
              data-testid="room-settings-name"
            />

            <label class="room-settings-modal__label" for="room-settings-description">
              {t!(i18n, room.description)}
              <span class="room-settings-modal__optional">{t!(i18n, common.optional)}</span>
            </label>
            <textarea
              id="room-settings-description"
              class="input"
              rows="2"
              maxlength="500"
              prop:value=move || description.get()
              on:input=move |ev| description.set(event_target_value(&ev))
              data-testid="room-settings-description"
            />

            <fieldset class="room-settings-modal__password">
              <legend class="room-settings-modal__legend">
                {t!(i18n, room.settings_password_section)}
              </legend>
              <button
                type="button"
                class="btn btn--ghost"
                on:click=move |_| password_open.set(true)
                data-testid="room-settings-set-password"
              >
                {t!(i18n, room.settings_password_set)}
              </button>
              <Show when=move || is_protected.get()>
                <button
                  type="button"
                  class="btn btn--danger"
                  on:click=move |_| confirm_clear_open.set(true)
                  data-testid="room-settings-clear-password"
                >
                  {t!(i18n, room.settings_password_clear)}
                </button>
              </Show>
            </fieldset>

            <Show when=move || error.get().is_some()>
              <p class="room-settings-modal__error" role="alert">
                {move || error.get().unwrap_or_default()}
              </p>
            </Show>
          </div>

          <footer class="modal-footer">
            <button
              type="button"
              class="btn btn--ghost"
              on:click=move |_| open.set(false)
              data-testid="room-settings-cancel"
            >
              {t!(i18n, common.cancel)}
            </button>
            <button
              type="submit"
              class="btn btn--primary"
              data-testid="room-settings-save"
            >
              {t!(i18n, room.settings_save)}
            </button>
          </footer>
        </form>
      </ModalWrapper>
    </Show>

    <Show when=move || password_open.get()>
      <PasswordPromptModal
        title=Signal::derive(move || t_string!(i18n, room.settings_password_set).to_string())
        mode=PasswordPromptMode::SetNew
        on_submit=on_password_submit
        on_cancel=on_password_cancel
      />
    </Show>

    <Show when=move || confirm_clear_open.get()>
      <ConfirmDialog
        title=Signal::derive(move || t_string!(i18n, room.settings_password_clear).to_string())
        description=Signal::derive(move ||
          t_string!(i18n, room.settings_password_confirm_clear).to_string())
        confirm_label=Signal::derive(move ||
          t_string!(i18n, room.settings_password_clear).to_string())
        tone=Signal::derive(|| ConfirmTone::Destructive)
        on_confirm=on_clear_confirm
        on_cancel=on_clear_cancel
      />
    </Show>
  }
}
