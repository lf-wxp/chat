//! Invite link panel + invite link join handling

use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use message::signal::SignalMessage;

use crate::{
  components::{Button, ButtonVariant},
  i18n::*,
  services::ws::WsClient,
  state,
};

/// Invite link panel — displays generated invite link and countdown timer
#[component]
pub(super) fn InviteLinkPanel() -> impl IntoView {
  let ui_state = state::use_ui_state();
  let countdown = RwSignal::new(0i64);
  let i18n = use_i18n();

  // Countdown timer
  Effect::new(move |_| {
    let state = ui_state.get();
    if let Some(expires_at) = state.invite_link_expires_at {
      let now = js_sys::Date::now() as i64;
      let remaining = (expires_at - now) / 1000;
      countdown.set(remaining.max(0));

      // Start 1-second interval countdown
      let handle = gloo_timers::callback::Interval::new(1_000, move || {
        let now = js_sys::Date::now() as i64;
        let remaining = (expires_at - now) / 1000;
        if remaining <= 0 {
          countdown.set(0);
          // Cleanup after expiration
          ui_state.update(|s| {
            s.invite_link_code = None;
            s.invite_link_expires_at = None;
          });
        } else {
          countdown.set(remaining);
        }
      });
      // Keep timer alive until component unmounts
      std::mem::forget(handle);
    }
  });

  view! {
      {move || {
        let state = ui_state.get();
        if let Some(ref code) = state.invite_link_code {
          let link = format!("{}?invite={}", web_sys::window().unwrap().location().origin().unwrap_or_default(), code);
          let link_for_copy = link.clone();
          let secs = countdown.get();
          view! {
            <div class="invite-link-panel">
              <div class="invite-link-header">
                <span class="invite-link-icon">"🔗"</span>
                <span class="invite-link-title">{t!(i18n, invite_link_title)}</span>
                <span class="invite-link-countdown">
                  {if secs > 0 {
                    format!("{}", t_string!(i18n, invite_expires_in).replace("{}", &secs.to_string()))
                  } else {
                    t_string!(i18n, invite_expired).to_string()
                  }}
                </span>
                <button
                  class="invite-link-close"
                  on:click=move |_| {
                    ui_state.update(|s| {
                      s.invite_link_code = None;
                      s.invite_link_expires_at = None;
                    });
                  }
                >
                  "✕"
                </button>
              </div>
              <div class="invite-link-body">
                <code class="invite-link-url">{link}</code>
                <Button
                  label=t_string!(i18n, common_copy).to_string()
                  variant=ButtonVariant::Primary
                  on_click=Callback::new(move |()| {
                    let link = link_for_copy.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                      let window = web_sys::window().unwrap();
                      let clipboard = window.navigator().clipboard();
                      let _ = wasm_bindgen_futures::JsFuture::from(
                        clipboard.write_text(&link)
                      ).await;
  web_sys::console::log_1(&"Invite link copied to clipboard".into());
                    });
                  })
                />
              </div>
            </div>
          }.into_any()
        } else {
          view! { <div></div> }.into_any()
        }
      }}
    }
}

/// Check if invite code parameter exists in URL and auto-join
pub fn check_invite_link_on_load() {
  let window = web_sys::window().unwrap();
  let search = window.location().search().unwrap_or_default();
  if let Some(code) = search.trim_start_matches('?').split('&').find_map(|pair| {
    let (key, val) = pair.split_once('=')?;

    if key == "invite" {
      Some(val.to_string())
    } else {
      None
    }
  }) {
    // Delay sending to wait for WebSocket connection establishment
    let code = code.clone();
    gloo_timers::callback::Timeout::new(2_000, move || {
      let ws = WsClient::use_client();
      let _ = ws.send(&SignalMessage::JoinByInviteLink { code });
    })
    .forget();
  }
}
