//! Password prompt shown when a user tries to join a protected room
//! (Req 4.2).
//!
//! The modal also doubles as the "password change" flow required by
//! Req 4.5a: the caller opts into two-factor password confirmation by
//! setting [`PasswordPromptMode::SetNew`], which reveals a second
//! password field and rejects submission unless the two entries match.

use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use message::error::validation::validate_room_password;

use crate::components::room::modal_wrapper::{ModalSize, ModalWrapper};
use crate::i18n;

/// Drives which fields are rendered inside the modal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordPromptMode {
  /// Single password field for joining a room.
  JoinExisting,
  /// Two password fields for creating or changing a password
  /// (Req 4.5a two-entry confirmation).
  SetNew,
}

/// Password prompt modal.
#[component]
#[allow(clippy::needless_pass_by_value)]
pub fn PasswordPromptModal(
  /// Title rendered inside the modal header.
  #[prop(into)]
  title: Signal<String>,
  /// Mode controlling single vs double password input.
  #[prop(optional)]
  mode: Option<PasswordPromptMode>,
  /// Fires with the confirmed password string. For `SetNew` mode the
  /// two fields must match before this is invoked.
  on_submit: Callback<String>,
  /// Fires on cancel (Escape, backdrop click, Cancel button).
  on_cancel: Callback<()>,
) -> impl IntoView {
  let i18n = i18n::use_i18n();
  let mode = mode.unwrap_or(PasswordPromptMode::JoinExisting);
  let pwd = RwSignal::new(String::new());
  let pwd_confirm = RwSignal::new(String::new());
  let error = RwSignal::new(Option::<String>::None);

  let validate_for_submit = move || -> Result<String, String> {
    let first = pwd.get();
    if let Err(e) = validate_room_password(&first) {
      return Err(e.message);
    }
    if first.is_empty() {
      return Err(t_string!(i18n, room.password_required).to_string());
    }
    if mode == PasswordPromptMode::SetNew {
      let second = pwd_confirm.get();
      if first != second {
        return Err(t_string!(i18n, room.password_mismatch).to_string());
      }
    }
    Ok(first)
  };

  let handle_submit = move |ev: leptos::ev::SubmitEvent| {
    ev.prevent_default();
    match validate_for_submit() {
      Ok(password) => {
        error.set(None);
        on_submit.run(password);
      }
      Err(msg) => error.set(Some(msg)),
    }
  };

  view! {
    <ModalWrapper
      on_close=on_cancel
      size=ModalSize::Small
      class="room-password-prompt"
      labelled_by="password-prompt-title"
      testid="password-prompt"
    >
      <form
        class="room-password-prompt__form"
        on:submit=handle_submit
      >
        <header class="modal-header">
          <h2 id="password-prompt-title" class="modal-title">{move || title.get()}</h2>
        </header>
        <div class="modal-body">
          <label class="room-password-prompt__label" for="room-password-input">
            {t!(i18n, room.password)}
          </label>
          <input
            id="room-password-input"
            class="input"
            type="password"
            autocomplete="current-password"
            prop:value=move || pwd.get()
            on:input=move |ev| pwd.set(event_target_value(&ev))
            data-testid="password-prompt-primary"
          />

          <Show when=move || mode == PasswordPromptMode::SetNew>
            <label class="room-password-prompt__label" for="room-password-confirm">
              {t!(i18n, room.password_confirm)}
            </label>
            <input
              id="room-password-confirm"
              class="input"
              type="password"
              autocomplete="new-password"
              prop:value=move || pwd_confirm.get()
              on:input=move |ev| pwd_confirm.set(event_target_value(&ev))
              data-testid="password-prompt-confirm"
            />
          </Show>

          <Show when=move || error.get().is_some()>
            <p class="room-password-prompt__error" role="alert">
              {move || error.get().unwrap_or_default()}
            </p>
          </Show>
        </div>
        <footer class="modal-footer">
          <button
            type="button"
            class="btn btn--ghost"
            on:click=move |_| on_cancel.run(())
            data-testid="password-prompt-cancel"
          >
            {t!(i18n, common.cancel)}
          </button>
          <button
            type="submit"
            class="btn btn--primary"
            data-testid="password-prompt-submit"
          >
            {t!(i18n, common.confirm)}
          </button>
        </footer>
      </form>
    </ModalWrapper>
  }
}
