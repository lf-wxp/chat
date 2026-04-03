//! Login/Register page

use leptos::prelude::*;
use leptos_i18n::t_string;
use leptos_router::NavigateOptions;
use leptos_router::hooks::use_navigate;

use message::signal::SignalMessage;

use crate::{
  components::{Button, Input, InputType},
  i18n::*,
  services::ws::WsClient,
  state,
};

/// Login/Register page component
#[component]
pub fn Login() -> impl IntoView {
  let username = RwSignal::new(String::new());
  let password = RwSignal::new(String::new());
  let is_register = RwSignal::new(false);
  let error_msg = RwSignal::new(Option::<String>::None);
  let loading = RwSignal::new(false);

  let i18n = use_i18n();

  let handle_submit = move |()| {
    let u = username.get_untracked();
    let p = password.get_untracked();

    if u.trim().is_empty() || p.trim().is_empty() {
      error_msg.set(Some(t_string!(i18n, auth_fields_required).to_string()));
      return;
    }

    loading.set(true);
    error_msg.set(None);

    // Send authentication message via WebSocket
    let ws = WsClient::use_client();
    let signal = if is_register.get_untracked() {
      SignalMessage::Register {
        username: u.clone(),
        password: p,
      }
    } else {
      SignalMessage::Login {
        username: u.clone(),
        password: p,
      }
    };

    match ws.send(&signal) {
      Ok(()) => {
        // Set username first, wait for server AuthSuccess response before completing authentication
        let user_state = state::use_user_state();
        user_state.update(|s| {
          s.username.clone_from(&u);
        });

        // Delay checking authentication result (allow time for server response)
        let loading_clone = loading;
        let error_msg_clone = error_msg;
        crate::utils::set_timeout(
          move || {
            let user_state = state::use_user_state();
            if user_state.get_untracked().authenticated {
              let navigate = use_navigate();
              navigate("/", NavigateOptions::default());
            } else {
              loading_clone.set(false);
              error_msg_clone.set(Some(t_string!(i18n, auth_timeout).to_string()));
            }
          },
          3000,
        );
      }
      Err(e) => {
        loading.set(false);
        error_msg.set(Some(format!("{}: {e}", t_string!(i18n, common_error))));
      }
    }
  };

  view! {
    <div class="page-login">
      <div class="login-card">
        <div class="login-header">
          <h1 class="login-title">"🌐 WebRTC Chat"</h1>
          <p class="login-subtitle">
            {move || if is_register.get() { t_string!(i18n, auth_create_account).to_string() } else { t_string!(i18n, auth_login_to_account).to_string() }}
          </p>
        </div>

        <div class="login-form">
          // Error message
          {move || error_msg.get().map(|msg| view! {
            <div class="login-error">{msg}</div>
          })}

          <Input
            label=t_string!(i18n, auth_username).to_string()
            placeholder=t_string!(i18n, auth_username_required).to_string()
            value=username
          />

          <Input
            label=t_string!(i18n, auth_password).to_string()
            placeholder=t_string!(i18n, auth_password_required).to_string()
            input_type=InputType::Password
            value=password
            on_enter=Callback::new(move |_| handle_submit(()))
          />

          <Button
            label=t_string!(i18n, auth_login_register_btn).to_string()
            full_width=true
            loading=loading.get_untracked()
            on_click=Callback::new(handle_submit)
          />

          <div class="login-switch">
            <span class="text-secondary text-sm">
              {move || if is_register.get() { t_string!(i18n, auth_have_account).to_string() } else { t_string!(i18n, auth_no_account).to_string() }}
            </span>
            <button
              class="link-btn"
              on:click=move |_| is_register.update(|v| *v = !*v)
              tabindex=0
            >
              {move || if is_register.get() { t_string!(i18n, auth_go_login).to_string() } else { t_string!(i18n, auth_go_register).to_string() }}
            </button>
          </div>
        </div>
      </div>
    </div>
  }
}
