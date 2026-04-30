//! Sidebar room section — compact room list for the sidebar panel.
//!
//! Shows available rooms with join/create actions in a sidebar-friendly
//! layout. Replaces the full-width `RoomListPanel` that used to live
//! in the main content area.

use icondata as i;
use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use leptos_icons::Icon;
use message::types::{RoomInfo, RoomType};

use crate::components::room::{CreateRoomModal, CreateRoomRequest, PasswordPromptModal};
use crate::error_handler::use_error_toast_manager;
use crate::i18n;
use crate::signaling::use_signaling_client;
use crate::state::use_app_state;

/// Compact room section rendered inside the sidebar.
#[component]
pub fn SidebarRoomSection() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let app_state = use_app_state();
  let signaling = use_signaling_client();
  let toast = use_error_toast_manager();

  let create_open = RwSignal::new(false);
  let password_target = RwSignal::new(Option::<RoomInfo>::None);

  let visible_rooms = Memo::new(move |_| app_state.rooms.with(|list| list.to_vec()));

  let signaling_for_create = signaling.clone();
  let toast_for_create = toast;
  let on_create_submit = Callback::new(move |req: CreateRoomRequest| {
    if let Err(e) = signaling_for_create.send_create_room(
      req.name.clone(),
      req.description.clone(),
      req.room_type,
      req.password.clone(),
    ) {
      web_sys::console::warn_1(&format!("[room] Failed to create room: {e}").into());
      toast_for_create.show_error_message_with_key(
        "ROM109",
        "error.rom109",
        t_string!(i18n, error.rom109),
      );
    }
  });

  let signaling_for_join = signaling.clone();
  let toast_for_join = toast;
  let handle_join: Callback<(RoomInfo, Option<String>)> =
    Callback::new(move |(room, password): (RoomInfo, Option<String>)| {
      if let Err(e) = signaling_for_join.send_join_room(room.room_id.clone(), password) {
        web_sys::console::warn_1(&format!("[room] Failed to join room: {e}").into());
        toast_for_join.show_error_message_with_key(
          "ROM109",
          "error.rom109",
          t_string!(i18n, error.rom109),
        );
      }
    });

  view! {
    <div class="sidebar-room-section" data-testid="sidebar-room-section">
      // Section header with create button
      <div class="sidebar-section-title sidebar-room-section__header">
        <span>{t!(i18n, room.rooms)}</span>
        <button
          type="button"
          class="sidebar-room-section__create-btn"
          on:click=move |_| create_open.set(true)
          aria-label=move || t_string!(i18n, room.create)
          title=move || t_string!(i18n, room.create)
        >
          <Icon icon=i::LuPlus />
        </button>
      </div>

      // Room list
      <Show
        when=move || !visible_rooms.get().is_empty()
        fallback=move || view! {
          <p class="sidebar-room-section__empty">{t!(i18n, room.empty_list)}</p>
        }
      >
        <For
          each=move || visible_rooms.get()
          key=|r: &RoomInfo| r.room_id.clone()
          children=move |room: RoomInfo| {
            let room_for_join = room.clone();
            let room_for_password = room.clone();
            let protected = room.is_password_protected();
            let is_full = room.is_full();
            let type_label = match room.room_type {
              RoomType::Chat => t_string!(i18n, room.room_type_chat),
              RoomType::Theater => t_string!(i18n, room.room_type_theater),
            };

            view! {
              <div class="sidebar-room-item" data-testid="sidebar-room-item">
                <div class="sidebar-room-item__info">
                  <div class="sidebar-room-item__name-row">
                    <span class="sidebar-room-item__name">{room.name.clone()}</span>
                    <Show when=move || protected>
                      <span class="sidebar-room-item__lock" aria-hidden="true">"🔒"</span>
                    </Show>
                  </div>
                  <span
                    class="sidebar-room-item__badge"
                    class:sidebar-room-item__badge--theater=room.room_type == RoomType::Theater
                  >{type_label}</span>
                </div>
                <button
                  type="button"
                  class="sidebar-room-item__join-btn"
                  disabled=is_full
                  aria-label=move || if is_full { t_string!(i18n, room.room_full) } else { t_string!(i18n, room.join) }
                  title=move || if is_full { t_string!(i18n, room.room_full) } else { t_string!(i18n, room.join) }
                  on:click=move |_| {
                    if protected {
                      password_target.set(Some(room_for_password.clone()));
                    } else {
                      handle_join.run((room_for_join.clone(), None));
                    }
                  }
                >
                  <Icon icon=i::LuLogIn />
                </button>
              </div>
            }
          }
        />
      </Show>

      <CreateRoomModal open=create_open on_submit=on_create_submit />

      <Show when=move || password_target.with(Option::is_some)>
        {
          let title = Signal::derive(move || {
            password_target
              .with(|t| t.as_ref().map(|r| r.name.clone()))
              .unwrap_or_default()
          });
          view! {
            <PasswordPromptModal
              title=title
              on_submit=Callback::new(move |pwd: String| {
                if let Some(room) = password_target.get() {
                  handle_join.run((room, Some(pwd)));
                }
                password_target.set(None);
              })
              on_cancel=Callback::new(move |()| password_target.set(None))
            />
          }
        }
      </Show>
    </div>
  }
}
