//! Connection invitation modal component

use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use message::signal::SignalMessage;

use crate::{
  components::{Avatar, AvatarSize, Button, ButtonVariant},
  i18n::*,
  services::ws::WsClient,
  state,
};

/// Connection invitation modal
#[component]
pub(crate) fn InviteReceivedModal(
  from_user_id: String,
  from_username: String,
  message: Option<String>,
) -> impl IntoView {
  let ui_state = state::use_ui_state();
  let i18n = use_i18n();

  let handle_accept = {
    let from_id = from_user_id.clone();
    move |()| {
      let user_state = state::use_user_state();
      let my_id = user_state.get_untracked().user_id.clone();
      let ws = WsClient::use_client();
      let _ = ws.send(&SignalMessage::InviteResponse {
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
      let _ = ws.send(&SignalMessage::InviteResponse {
        from: my_id,
        to: from_id.clone(),
        accepted: false,
      });
      ui_state.update(|s| s.active_modal = None);
    }
  };

  let handle_overlay_click = move |_: web_sys::MouseEvent| {
    // Clicking overlay does not close the invite modal, explicit action required
  };

  let username_for_avatar = from_username.clone();
  let username_display = from_username.clone();

  view! {
    <div class="modal-overlay" on:click=handle_overlay_click role="dialog" aria-modal="true" aria-label=t_string!(i18n, invite_connection)>
      <div class="modal-content invite-modal" on:click=|ev| ev.stop_propagation()>
        <div class="invite-modal-header">
          <div class="invite-modal-icon">"🔗"</div>
          <h3 class="modal-title">{t!(i18n, invite_connection)}</h3>
        </div>
        <div class="invite-modal-body">
          <div class="invite-user-info">
            <Avatar username=username_for_avatar size=AvatarSize::Medium online=true />
            <div class="invite-user-name">{username_display}</div>
          </div>
          <p class="invite-message">
            {message.unwrap_or_else(|| t_string!(i18n, invite_wants_to_connect).to_string())}
          </p>
        </div>
        <div class="invite-modal-actions">
          <Button
            label=t_string!(i18n, common_reject).to_string()
            variant=ButtonVariant::Secondary
            on_click=Callback::new(handle_reject)
          />
          <Button
            label=t_string!(i18n, common_accept).to_string()
            variant=ButtonVariant::Primary
            on_click=Callback::new(handle_accept)
          />
        </div>
      </div>
    </div>
  }
}
