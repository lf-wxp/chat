//! Nickname editor (Req 15.1).
//!
//! Inline editor shown from the settings drawer or the top-right user
//! menu. Validates the new nickname via
//! `message::error::validation::validate_nickname` and, on success,
//! updates local auth state, persists it to localStorage and
//! broadcasts the change via signaling.

use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use message::error::validation::validate_nickname;

use crate::error_handler::use_error_toast_manager;
use crate::i18n;
use crate::signaling::use_signaling_client;
use crate::state::use_app_state;
use crate::utils;

/// Nickname editor panel. Emits a real-time validation error beneath
/// the input and only enables the Save button when the current value
/// differs from the persisted nickname and passes validation.
#[component]
pub fn NicknameEditor() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let app_state = use_app_state();
  let signaling = use_signaling_client();
  let toast = use_error_toast_manager();

  let current_nickname = Memo::new(move |_| {
    app_state
      .auth
      .with(|a| a.as_ref().map(|s| s.nickname.clone()).unwrap_or_default())
  });

  let draft = RwSignal::new(current_nickname.get_untracked());
  let error = RwSignal::new(Option::<String>::None);

  // Keep the draft in sync when auth state changes externally.
  Effect::new(move |_| {
    draft.set(current_nickname.get());
  });

  let validation_memo = Memo::new(move |_| {
    let value = draft.get();
    if value.is_empty() {
      // Empty input reverts to the username — Req 15.1 §8.
      return None;
    }
    validate_nickname(&value).err().map(|e| e.message)
  });

  let is_dirty = Memo::new(move |_| draft.get() != current_nickname.get());
  let can_save = Memo::new(move |_| is_dirty.get() && validation_memo.get().is_none());

  let handle_save = move || {
    let raw = draft.get();
    let trimmed = raw.trim();
    let final_nickname = if trimmed.is_empty() {
      // Revert to username when cleared (Req 15.1.8).
      app_state
        .auth
        .with_untracked(|a| a.as_ref().map(|s| s.username.clone()))
        .unwrap_or_default()
    } else {
      // Only validate when the user actually entered a custom nickname.
      // The "revert to username" path (trimmed.is_empty) skips
      // validation because usernames are pre-validated by the auth
      // system and validate_nickname would always reject the empty
      // string that the else-branch would have produced.
      if let Err(e) = validate_nickname(trimmed) {
        error.set(Some(e.message));
        return;
      }
      trimmed.to_string()
    };
    error.set(None);
    // Reflect the canonical (trimmed) value back in the input so the
    // user sees what was actually persisted.
    if draft.get_untracked() != final_nickname {
      draft.set(final_nickname.clone());
    }
    // Update local auth signal so the new nickname renders immediately.
    app_state.auth.update(|maybe_state| {
      if let Some(state) = maybe_state {
        state.nickname = final_nickname.clone();
      }
    });
    // Persist to localStorage so a refresh retains the change (R10.9).
    utils::save_to_local_storage("nickname", &final_nickname);

    if let Err(e) = signaling.send_nickname_change(final_nickname) {
      web_sys::console::warn_1(&format!("[room] Failed to broadcast nickname change: {e}").into());
      toast.show_error_message_with_key("ROM114", "error.rom114", t_string!(i18n, error.rom114));
    }
  };

  view! {
    <section class="nickname-editor" data-testid="nickname-editor">
      <label class="nickname-editor__label" for="nickname-editor-input">
        {t!(i18n, auth.nickname)}
      </label>
      <input
        id="nickname-editor-input"
        class="input"
        type="text"
        maxlength="20"
        aria-describedby="nickname-editor-help"
        prop:value=move || draft.get()
        on:input=move |ev| draft.set(event_target_value(&ev))
        placeholder=move || t_string!(i18n, room.nickname_placeholder)
        data-testid="nickname-editor-input"
      />
      <p id="nickname-editor-help" class="nickname-editor__hint">
        {t!(i18n, room.nickname_help)}
      </p>
      <Show when=move || validation_memo.get().is_some()>
        <p class="nickname-editor__error" role="alert" data-testid="nickname-editor-error">
          {move || validation_memo.get().unwrap_or_default()}
        </p>
      </Show>
      <Show when=move || error.get().is_some()>
        <p class="nickname-editor__error" role="alert">
          {move || error.get().unwrap_or_default()}
        </p>
      </Show>
      <div class="nickname-editor__actions">
        <button
          type="button"
          class="btn btn--primary"
          disabled=move || !can_save.get()
          on:click=move |_| handle_save()
          data-testid="nickname-editor-save"
        >
          {t!(i18n, common.save)}
        </button>
      </div>
    </section>
  }
}
