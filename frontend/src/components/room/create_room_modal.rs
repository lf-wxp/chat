//! Create-room modal (Req 4.1).
//!
//! Collects the room name, optional description, room type
//! (Chat / Theater) and an optional password. Performs client-side
//! validation via the shared `message::error::validation` functions so
//! the user sees an inline message before the request is dispatched.

use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use message::error::validation::{
  validate_room_description, validate_room_name, validate_room_password,
};
use message::types::RoomType;

use crate::components::room::modal_wrapper::{ModalSize, ModalWrapper};
use crate::components::room::utils::event_target_checked;
use crate::i18n;
use icondata as i;
use leptos_icons::Icon;

/// Payload emitted by [`CreateRoomModal`] when the user submits.
#[derive(Debug, Clone)]
pub struct CreateRoomRequest {
  /// Room display name.
  pub name: String,
  /// Optional short description shown in the room list.
  pub description: String,
  /// Chat or Theater room type.
  pub room_type: RoomType,
  /// Optional password (empty string = no password).
  pub password: Option<String>,
}

/// Create-room modal.
#[component]
#[allow(clippy::needless_pass_by_value)]
pub fn CreateRoomModal(
  /// Reactive flag driving visibility. The modal closes itself by
  /// setting this to `false` after successful submit or cancel.
  open: RwSignal<bool>,
  /// Invoked when the user submits a valid form.
  on_submit: Callback<CreateRoomRequest>,
) -> impl IntoView {
  let i18n = i18n::use_i18n();

  let name = RwSignal::new(String::new());
  let description = RwSignal::new(String::new());
  let password = RwSignal::new(String::new());
  let confirm_password = RwSignal::new(String::new());
  let room_type = RwSignal::new(RoomType::Chat);
  let use_password = RwSignal::new(false);
  let error = RwSignal::new(Option::<String>::None);

  // Reset form fields whenever the modal is reopened.
  Effect::new(move |_| {
    if open.get() {
      name.set(String::new());
      description.set(String::new());
      password.set(String::new());
      confirm_password.set(String::new());
      room_type.set(RoomType::Chat);
      use_password.set(false);
      error.set(None);
    }
  });

  let close_modal = move || open.set(false);

  let handle_submit = move |ev: leptos::ev::SubmitEvent| {
    ev.prevent_default();
    let current_name = name.get();
    let current_description = description.get();

    if let Err(e) = validate_room_name(&current_name) {
      error.set(Some(e.message));
      return;
    }
    if !current_description.is_empty()
      && let Err(e) = validate_room_description(&current_description)
    {
      error.set(Some(e.message));
      return;
    }

    let pwd_opt = if use_password.get() {
      let current_pwd = password.get();
      if let Err(e) = validate_room_password(&current_pwd) {
        error.set(Some(e.message));
        return;
      }
      if current_pwd.is_empty() {
        error.set(Some(t_string!(i18n, room.password_required).to_string()));
        return;
      }
      if current_pwd != confirm_password.get() {
        error.set(Some(t_string!(i18n, room.password_mismatch).to_string()));
        return;
      }
      Some(current_pwd)
    } else {
      None
    };

    error.set(None);
    on_submit.run(CreateRoomRequest {
      name: current_name,
      description: current_description,
      room_type: room_type.get(),
      password: pwd_opt,
    });
    close_modal();
  };

  view! {
    <Show when=move || open.get()>
      <ModalWrapper
        on_close=Callback::new(move |()| close_modal())
        size=ModalSize::Medium
        class="create-room-modal"
        labelled_by="create-room-title"
        testid="create-room-modal"
      >
        <form
          class="create-room-modal__form"
          on:submit=handle_submit
        >
          <header class="modal-header">
            <h2 id="create-room-title" class="modal-title">{t!(i18n, room.create)}</h2>
            <button
              type="button"
              class="modal-close"
              aria-label=move || t_string!(i18n, common.close)
              on:click=move |_| close_modal()
            >
              <Icon icon=i::LuX />
            </button>
          </header>

          <div class="modal-body create-room-modal__body">
            <fieldset class="create-room-modal__types" aria-label=move || t_string!(i18n, room.room_type)>
              <legend class="create-room-modal__legend">{t!(i18n, room.room_type)}</legend>
              <label class="create-room-modal__type-option">
                <input
                  type="radio"
                  name="room-type"
                  prop:checked=move || room_type.get() == RoomType::Chat
                  on:change=move |_| room_type.set(RoomType::Chat)
                  data-testid="room-type-chat"
                />
                <span>{t!(i18n, room.room_type_chat)}</span>
              </label>
              <label class="create-room-modal__type-option">
                <input
                  type="radio"
                  name="room-type"
                  prop:checked=move || room_type.get() == RoomType::Theater
                  on:change=move |_| room_type.set(RoomType::Theater)
                  data-testid="room-type-theater"
                />
                <span>{t!(i18n, room.room_type_theater)}</span>
              </label>
            </fieldset>

            <label class="create-room-modal__label" for="create-room-name">
              {t!(i18n, room.room_name)}
            </label>
            <input
              id="create-room-name"
              class="input"
              type="text"
              maxlength="100"
              required
              prop:value=move || name.get()
              on:input=move |ev| name.set(event_target_value(&ev))
              data-testid="create-room-name"
            />

            <label class="create-room-modal__label" for="create-room-description">
              {t!(i18n, room.description)}
              <span class="create-room-modal__optional">{t!(i18n, common.optional)}</span>
            </label>
            <textarea
              id="create-room-description"
              class="input"
              rows="2"
              maxlength="500"
              prop:value=move || description.get()
              on:input=move |ev| description.set(event_target_value(&ev))
              data-testid="create-room-description"
            />

            <label class="create-room-modal__checkbox">
              <input
                type="checkbox"
                prop:checked=move || use_password.get()
                on:change=move |ev| use_password.set(event_target_checked(&ev))
                data-testid="create-room-password-toggle"
              />
              <span>{t!(i18n, room.use_password)}</span>
            </label>

            <Show when=move || use_password.get()>
              <label class="create-room-modal__label" for="create-room-password">
                {t!(i18n, room.password)}
              </label>
              <input
                id="create-room-password"
                class="input"
                type="password"
                autocomplete="new-password"
                maxlength="64"
                prop:value=move || password.get()
                on:input=move |ev| password.set(event_target_value(&ev))
                data-testid="create-room-password"
              />
              <label class="create-room-modal__label" for="create-room-password-confirm">
                {t!(i18n, room.password_confirm)}
              </label>
              <input
                id="create-room-password-confirm"
                class="input"
                type="password"
                autocomplete="new-password"
                maxlength="64"
                prop:value=move || confirm_password.get()
                on:input=move |ev| confirm_password.set(event_target_value(&ev))
                data-testid="create-room-password-confirm"
              />
            </Show>

            <Show when=move || error.get().is_some()>
              <p class="create-room-modal__error" role="alert">
                {move || error.get().unwrap_or_default()}
              </p>
            </Show>
          </div>

          <footer class="modal-footer">
            <button
              type="button"
              class="btn btn--ghost"
              on:click=move |_| close_modal()
              data-testid="create-room-cancel"
            >
              {t!(i18n, common.cancel)}
            </button>
            <button
              type="submit"
              class="btn btn--primary"
              data-testid="create-room-submit"
            >
              {t!(i18n, room.create)}
            </button>
          </footer>
        </form>
      </ModalWrapper>
    </Show>
  }
}
