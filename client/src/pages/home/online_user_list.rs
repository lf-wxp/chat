//! Online user list component

use leptos::prelude::*;
use leptos_i18n::t_string;
use message::signal::{InviteType, SignalMessage};

use crate::{
  components::{Avatar, AvatarSize, Button, ButtonVariant, EmptyState, Input, InputType},
  i18n::*,
  services::ws::WsClient,
  state,
};

/// Online user list
#[component]
pub(super) fn OnlineUserList() -> impl IntoView {
  let online_state = state::use_online_users_state();
  let search = RwSignal::new(String::new());
  let i18n = use_i18n();

  view! {
      <div class="online-users">
        <div class="p-2">
          <Input
            placeholder=t_string!(i18n, common_search_users).to_string()
            input_type=InputType::Search
            value=search
          />
        </div>
        <div class="online-users-list">
          {move || {
            let query = search.get().to_lowercase();
            let users: Vec<_> = online_state.get().users.iter()
              .filter(|u| query.is_empty() || u.username.to_lowercase().contains(&query))
              .cloned()
              .collect();

            if users.is_empty() {
              view! {
                <EmptyState
                  icon="👥"
                  title=t_string!(i18n, nav_no_online_users).to_string()
                  description=""
                />
              }.into_any()
            } else {
              users.iter().map(|user| {
                let username = user.username.clone();
                let user_id = user.user_id.clone();
                let is_online = user.status == message::signal::UserStatus::Online;
                let username_for_aria = username.clone();
                let username_for_avatar = username.clone();
                let user_id_for_profile = user_id.clone();
                view! {
                  <div class="user-list-item" tabindex=0 aria-label=t_string!(i18n, user_aria).to_string().replace("{}", &username_for_aria)>
                    <div
                      class="user-list-item-clickable"
                      on:click={
                        let uid = user_id_for_profile.clone();
                        move |_| {
                          let ui_state = state::use_ui_state();
                          ui_state.update(|s| {
                            s.active_modal = Some(state::ModalType::UserProfile(uid.clone()));
                          });
                        }
                      }
                      style="cursor:pointer;display:flex;align-items:center;gap:12px;flex:1;min-width:0;"
                    >
                      <Avatar username=username_for_avatar size=AvatarSize::Small online=is_online />
                      <div class="flex-1">
                        <div class="font-medium truncate">{username}</div>
                      </div>
                    </div>
                    <Button
                      label=t_string!(i18n, invite_send).to_string()
                      variant=ButtonVariant::Ghost
                      on_click={
                        let user_id = user_id.clone();
                        Callback::new(move |()| {
                          let user_state = state::use_user_state();
                          let my_id = user_state.get_untracked().user_id.clone();
                          let ws = WsClient::use_client();
                          let _ = ws.send(&SignalMessage::ConnectionInvite {
                            from: my_id,
                            to: user_id.clone(),
                            message: None,
                            invite_type: InviteType::Chat,
                          });
                          web_sys::console::log_1(&format!("Connection invitation sent to: {user_id}").into());
                        })
                      }
                    />
                  </div>
                }
              }).collect_view().into_any()
            }
          }}
        </div>
      </div>
    }
}
