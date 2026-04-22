//! Authentication page component.
//!
//! Shows login or registration form based on user selection.
//! Displayed when the user is not authenticated.

use leptos::prelude::*;
use leptos_i18n::t;

use crate::auth::{login_form::LoginForm, register_form::RegisterForm};
use crate::i18n;

/// Auth page mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthMode {
  Login,
  Register,
}

/// Authentication page component.
#[component]
pub fn AuthPage() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let (mode, set_mode) = signal(AuthMode::Login);

  view! {
    <div class="auth-page flex items-center justify-center min-h-screen">
      <div class="auth-card">
        <div class="auth-card-header">
          <h1 class="auth-card-title">{t!(i18n, app.title)}</h1>
        </div>

        <div class="auth-card-body">
          <Show
            when=move || mode.get() == AuthMode::Login
            fallback=move || view! { <RegisterForm /> }
          >
            <LoginForm />
          </Show>
        </div>

        <div class="auth-card-footer">
          <Show
            when=move || mode.get() == AuthMode::Login
            fallback=move || {
              view! {
                <span class="auth-switch-text">{t!(i18n, auth.has_account)}</span>
                <button
                  class="btn btn-link"
                  on:click=move |_| set_mode.set(AuthMode::Login)
                >
                  {t!(i18n, auth.login)}
                </button>
              }
            }
          >
            <span class="auth-switch-text">{t!(i18n, auth.no_account)}</span>
            <button
              class="btn btn-link"
              on:click=move |_| set_mode.set(AuthMode::Register)
            >
              {t!(i18n, auth.register)}
            </button>
          </Show>
        </div>
      </div>
    </div>
  }
}
