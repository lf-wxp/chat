//! Incoming call modal component

use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use message::signal::SignalMessage;

use crate::{
  components::{Avatar, AvatarSize},
  i18n::*,
  services::ws::WsClient,
  state,
};

/// Incoming call modal
#[component]
pub(crate) fn IncomingCallModal(
  from_user_id: String,
  from_username: String,
  is_video: bool,
) -> impl IntoView {
  let ui_state = state::use_ui_state();
  let i18n = use_i18n();

  let handle_accept = {
    let from_id = from_user_id.clone();
    move |()| {
      let user_state = state::use_user_state();
      let my_id = user_state.get_untracked().user_id.clone();
      let ws = WsClient::use_client();
      let _ = ws.send(&SignalMessage::CallResponse {
        from: my_id,
        to: from_id.clone(),
        accepted: true,
      });
      ui_state.update(|s| s.active_modal = None);
    }
  };

  let handle_reject = {
    let from_id = from_user_id.clone();
    move |()| {
      let user_state = state::use_user_state();
      let my_id = user_state.get_untracked().user_id.clone();
      let ws = WsClient::use_client();
      let _ = ws.send(&SignalMessage::CallResponse {
        from: my_id,
        to: from_id.clone(),
        accepted: false,
      });
      ui_state.update(|s| s.active_modal = None);
    }
  };

  let call_type_text = if is_video {
    t_string!(i18n, call_video_type)
  } else {
    t_string!(i18n, call_voice_type)
  };
  let call_icon = if is_video { "📹" } else { "📞" };
  let username_for_avatar = from_username.clone();
  let username_display = from_username.clone();

  view! {
    <div class="modal-overlay call-overlay" role="dialog" aria-modal="true" aria-label=t_string!(i18n, call_incoming)>
      <div class="modal-content call-modal" on:click=|ev| ev.stop_propagation()>
        <div class="call-modal-header">
          <div class="call-modal-icon call-pulse">{call_icon}</div>
          <h3 class="modal-title">{call_type_text}</h3>
        </div>
        <div class="call-modal-body">
          <div class="call-user-info">
            <div class="call-avatar-ring">
              <Avatar username=username_for_avatar size=AvatarSize::Large online=true />
            </div>
            <div class="call-user-name">{username_display}</div>
            <div class="call-status-text">{t!(i18n, call_calling_you)}</div>
          </div>
        </div>
        <div class="call-modal-actions">
          <button
            class="call-btn call-btn-reject"
            on:click=move |_| handle_reject.clone()(())
            tabindex=0
            aria-label=move || t_string!(i18n, call_reject_call)
          >
            <span class="call-btn-icon">"✕"</span>
            <span class="call-btn-label">{t!(i18n, call_reject)}</span>
          </button>
          <button
            class="call-btn call-btn-accept"
            on:click=move |_| handle_accept.clone()(())
            tabindex=0
            aria-label=move || t_string!(i18n, call_accept_call)
          >
            <span class="call-btn-icon">"✓"</span>
            <span class="call-btn-label">{t!(i18n, call_accept)}</span>
          </button>
        </div>
      </div>
    </div>
  }
}
