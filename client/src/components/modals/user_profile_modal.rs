//! User profile card modal component

use leptos::prelude::*;
use leptos_i18n::t_string;
use message::signal::SignalMessage;

use crate::{
  components::{Avatar, AvatarSize, Button, ButtonVariant},
  i18n::*,
  services::ws::WsClient,
  state,
};

/// User profile card modal
#[component]
pub(crate) fn UserProfileModal(user_id: String) -> impl IntoView {
  let ui_state = state::use_ui_state();
  let online_state = state::use_online_users_state();
  let i18n = use_i18n();

  // Look up user info from online users list
  let uid = user_id.clone();
  let user_info = move || {
    online_state
      .get()
      .users
      .iter()
      .find(|u| u.user_id == uid)
      .cloned()
  };

  let close = move |_: web_sys::MouseEvent| {
    ui_state.update(|s| s.active_modal = None);
  };

  // user_id for action buttons (cloned for use in reactive closure)
  let uid_for_actions = user_id.clone();

  view! {
    <div class="modal-overlay" on:click=close role="dialog" aria-modal="true" aria-label=t_string!(i18n, profile_user_info)>
      <div class="modal-content user-profile-card" on:click=|ev| ev.stop_propagation()>
        {move || {
          if let Some(user) = user_info() {
            let is_online = user.status == message::signal::UserStatus::Online;
            let status_text = match user.status {
              message::signal::UserStatus::Online => t_string!(i18n, common_online),
              message::signal::UserStatus::Offline => t_string!(i18n, common_offline),
              message::signal::UserStatus::Busy => t_string!(i18n, common_busy),
              message::signal::UserStatus::Away => t_string!(i18n, common_away),
            };
            let status_class = match user.status {
              message::signal::UserStatus::Online => "profile-status-online",
              message::signal::UserStatus::Busy => "profile-status-busy",
              message::signal::UserStatus::Away => "profile-status-away",
              message::signal::UserStatus::Offline => "profile-status-offline",
            };
            let username_display = user.username.clone();
            let username_avatar = user.username.clone();
            let user_id_display = user.user_id.clone();

            view! {
              // Card top banner
              <div class="profile-card-banner"></div>

              // Avatar area
              <div class="profile-card-avatar-area">
                <div class="profile-card-avatar-ring">
                  <Avatar username=username_avatar size=AvatarSize::Large online=is_online />
                </div>
              </div>

              // User info
              <div class="profile-card-info">
                <h3 class="profile-card-name">{username_display}</h3>
                <div class=format!("profile-card-status {}", status_class)>
                  <span class="profile-status-dot"></span>
                  {status_text}
                </div>
                <div class="profile-card-id">
                  <span class="profile-card-id-label">"ID: "</span>
                  <span class="profile-card-id-value">{user_id_display}</span>
                </div>
              </div>

              // Action buttons
              <div class="profile-card-actions">
                <button
                  class="profile-action-btn profile-action-chat"
                  on:click={
                    let target = uid_for_actions.clone();
                    move |_| {
                      let my_id = state::use_user_state().get_untracked().user_id.clone();
                      let ws = WsClient::use_client();
                      let _ = ws.send(&SignalMessage::ConnectionInvite {
                        from: my_id,
                        to: target.clone(),
                        message: None,
                        invite_type: message::signal::InviteType::Chat,
                      });
                      state::use_ui_state().update(|s| s.active_modal = None);
                    }
                  }
                  title=t_string!(i18n, profile_start_chat)
                >
                  <span class="profile-action-icon">"💬"</span>
                  <span class="profile-action-label">{t_string!(i18n, profile_chat)}</span>
                </button>
                <button
                  class="profile-action-btn profile-action-audio"
                  on:click={
                    let target = uid_for_actions.clone();
                    move |_| {
                      let my_id = state::use_user_state().get_untracked().user_id.clone();
                      let ws = WsClient::use_client();
                      let _ = ws.send(&SignalMessage::CallInvite {
                        from: my_id,
                        to: vec![target.clone()],
                        media_type: message::types::MediaType::Audio,
                      });
                      state::use_ui_state().update(|s| s.active_modal = None);
                    }
                  }
                  title=t_string!(i18n, profile_voice_call)
                >
                  <span class="profile-action-icon">"🎤"</span>
                  <span class="profile-action-label">{t_string!(i18n, profile_voice)}</span>
                </button>
                <button
                  class="profile-action-btn profile-action-video"
                  on:click={
                    let target = uid_for_actions.clone();
                    move |_| {
                      let my_id = state::use_user_state().get_untracked().user_id.clone();
                      let ws = WsClient::use_client();
                      let _ = ws.send(&SignalMessage::CallInvite {
                        from: my_id,
                        to: vec![target.clone()],
                        media_type: message::types::MediaType::Video,
                      });
                      state::use_ui_state().update(|s| s.active_modal = None);
                    }
                  }
                  title=t_string!(i18n, profile_video_call)
                >
                  <span class="profile-action-icon">"📹"</span>
                  <span class="profile-action-label">{t_string!(i18n, profile_video)}</span>
                </button>
              </div>

              // Close button
              <button
                class="profile-card-close"
                on:click=move |_| {
                  let ui = state::use_ui_state();
                  ui.update(|s| s.active_modal = None);
                }
                aria-label=t_string!(i18n, common_close)
              >
                "✕"
              </button>
            }.into_any()
          } else {
            view! {
              <div class="profile-card-empty">
                <span class="profile-card-empty-icon">"👤"</span>
                <p>{t_string!(i18n, profile_unavailable)}</p>
                <Button
                  label=t_string!(i18n, common_close).to_string()
                  variant=ButtonVariant::Secondary
                  on_click=Callback::new(move |()| {
                    ui_state.update(|s| s.active_modal = None);
                  })
                />
              </div>
            }.into_any()
          }
        }}
      </div>
    </div>
  }
}
