//! Room list component

use leptos::prelude::*;
use leptos_i18n::t_string;
use message::signal::SignalMessage;

use crate::{
  components::{Button, ButtonVariant, EmptyState},
  i18n::*,
  services::ws::WsClient,
  state,
};

/// Room list
#[component]
pub(super) fn RoomList() -> impl IntoView {
  let room_state = state::use_room_state();
  let i18n = use_i18n();

  view! {
      <div class="room-list">
        <div class="p-2">
          <Button
            label=t_string!(i18n, room_create).to_string()
            full_width=true
            on_click=Callback::new(|()| {
              let ui_state = state::use_ui_state();
              ui_state.update(|s| {
                s.active_modal = Some(state::ModalType::CreateRoom);
              });
            })
          />
        </div>
        <div class="room-list-items">
          {move || {
            let rooms = &room_state.get().rooms;
            if rooms.is_empty() {
              view! {
                <EmptyState
                  icon="🏠"
                  title=t_string!(i18n, room_no_rooms).to_string()
                  description=""
                />
              }.into_any()
            } else {
              rooms.iter().map(|room| {
                let name = room.name.clone();
                let member_count = room.member_count;
                let max_members = room.max_members;
                let has_password = room.has_password;
                let room_type = room.room_type;
                let name_for_aria = name.clone();
                view! {
  <div class="room-list-item" tabindex=0 aria-label=t_string!(i18n, room_aria).to_string().replace("{}", &name_for_aria)>
                    <div class="room-list-item-icon">
                      {if room_type == message::signal::RoomType::Theater { "🎬" } else { "💬" }}
                    </div>
                    <div class="flex-1">
                      <div class="font-medium truncate">
                        {name}
                        {if has_password { " 🔒" } else { "" }}
                      </div>
                      <div class="text-secondary text-xs">
                        {format!("{member_count}/{max_members} members")}
                      </div>
                    </div>
                    <Button
                      label=t_string!(i18n, room_join).to_string()
                      variant=ButtonVariant::Ghost
                      on_click={
                        let room_id = room.room_id.clone();
                        let has_password = room.has_password;
                        Callback::new(move |()| {
                          let ws = WsClient::use_client();
                          // If room has password, could show input dialog in future; for now join directly
                          let _ = ws.send(&SignalMessage::JoinRoom {
                            room_id: room_id.clone(),
                            password: None,
                          });
  if has_password {
                            web_sys::console::log_1(&"This room requires a password, attempted to join without password".into());
                          }
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
