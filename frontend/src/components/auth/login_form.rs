//! Login form component.

use leptos::prelude::*;
use leptos_i18n::{t, t_string};

use crate::auth;
use crate::i18n;
use crate::state::use_app_state;
use message::error::validation::validate_username;

/// Login form component.
#[component]
pub fn LoginForm() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let app_state = use_app_state();

  let (username, set_username) = signal(String::new());
  let (password, set_password) = signal(String::new());
  let (error, set_error) = signal(None::<String>);
  let (loading, set_loading) = signal(false);

  let on_submit = move |ev: leptos::ev::SubmitEvent| {
    ev.prevent_default();
    set_error.set(None);

    let username_val = username.get();
    let password_val = password.get();

    // Client-side username validation (same as RegisterForm) to avoid
    // unnecessary network requests for obviously invalid input (Issue-1 fix).
    if let Err(err) = validate_username(&username_val) {
      set_error.set(Some(err.message));
      return;
    }
    if password_val.trim().is_empty() {
      set_error.set(Some(t_string!(i18n, auth.password_required).to_string()));
      return;
    }
    // P2-3 fix: Add client-side password length check consistent with
    // RegisterForm. While the server is the authoritative source for
    // password policy, a client-side check here avoids unnecessary
    // network requests for passwords that are obviously too short.
    if password_val.len() < auth::MIN_PASSWORD_LENGTH {
      set_error.set(Some(t_string!(i18n, auth.password_too_short).to_string()));
      return;
    }

    set_loading.set(true);
    auth::login(username_val, password_val, app_state, move |result| {
      set_loading.set(false);
      if !result.success {
        set_error.set(result.error);
      }
    });
  };

  view! {
    <form class="auth-form" on:submit=on_submit>
      <h2 class="auth-form-title">{t!(i18n, auth.login)}</h2>

      <Show when=move || error.get().is_some()>
        <div class="auth-form-error" role="alert" id="login-error">
          {move || error.get().unwrap_or_default()}
        </div>
      </Show>

      <div class="form-field">
        <label for="login-username">{t!(i18n, auth.username)}</label>
        <input
          id="login-username"
          type="text"
          class="input"
          placeholder=move || t_string!(i18n, auth.username_placeholder)
          prop:value=move || username.get()
          on:input=move |ev| set_username.set(event_target_value(&ev))
          autocomplete="username"
          minlength="3"
          maxlength="20"
          pattern="[a-zA-Z_][a-zA-Z0-9_]*"
          aria-describedby=move || error.get().is_some().then_some("login-error")
          required
        />
      </div>

      <div class="form-field">
        <label for="login-password">{t!(i18n, auth.password)}</label>
        <input
          id="login-password"
          type="password"
          class="input"
          placeholder=move || t_string!(i18n, auth.password_placeholder)
          prop:value=move || password.get()
          on:input=move |ev| set_password.set(event_target_value(&ev))
          autocomplete="current-password"
          maxlength="128"
          aria-describedby=move || error.get().is_some().then_some("login-error")
          required
        />
      </div>

      <button
        type="submit"
        class="btn btn-primary btn-block"
        disabled=move || loading.get()
      >
        {move || if loading.get() { t_string!(i18n, auth.logging_in) } else { t_string!(i18n, auth.login) }}
      </button>
    </form>
  }
}
