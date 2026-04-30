//! Room list panel (Req 4.9).
//!
//! Renders the currently visible rooms (server-pushed via
//! `RoomListUpdate`) along with quick actions to join, leave or
//! create new rooms. The panel lives in the home area and is shown
//! whenever the user is not actively looking at a direct message or
//! room conversation.

use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use message::types::{RoomInfo, RoomType};

use crate::components::room::create_room_modal::{CreateRoomModal, CreateRoomRequest};
use crate::components::room::password_prompt_modal::PasswordPromptModal;
use crate::components::room::utils::interpolate_member_count;
use crate::error_handler::use_error_toast_manager;
use crate::i18n;
use crate::signaling::use_signaling_client;
use crate::state::use_app_state;

/// Top-level room list panel.
#[component]
pub fn RoomListPanel() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let app_state = use_app_state();
  let signaling = use_signaling_client();
  let toast = use_error_toast_manager();

  let query = RwSignal::new(String::new());
  let create_open = RwSignal::new(false);
  let password_target = RwSignal::new(Option::<RoomInfo>::None);

  let visible_rooms = Memo::new(move |_| {
    let q = query.get().to_lowercase();
    app_state.rooms.with(|list| {
      list
        .iter()
        .filter(|room| {
          if q.is_empty() {
            return true;
          }
          room.name.to_lowercase().contains(&q) || room.description.to_lowercase().contains(&q)
        })
        .cloned()
        .collect::<Vec<_>>()
    })
  });

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
      // Do NOT set active_conversation here — the RoomJoined handler
      // will call ensure_room_conversation() which does it after the
      // server confirms the join.
    });

  view! {
    <section class="room-list-panel" data-testid="room-list-panel">
      <header class="room-list-panel__header">
        <h2 class="room-list-panel__title">{t!(i18n, room.rooms)}</h2>
        <button
          type="button"
          class="btn btn--primary room-list-panel__create-btn"
          on:click=move |_| create_open.set(true)
          data-testid="room-list-create"
        >
          {t!(i18n, room.create)}
        </button>
      </header>

      <div class="room-list-panel__search">
        <input
          type="search"
          class="input"
          placeholder=move || t_string!(i18n, common.search)
          prop:value=move || query.get()
          on:input=move |ev| query.set(event_target_value(&ev))
          data-testid="room-list-search"
        />
      </div>

      <Show
        when=move || !visible_rooms.get().is_empty()
        fallback=move || view! {
          <p class="room-list-panel__empty">{t!(i18n, room.empty_list)}</p>
        }
      >
        <ul class="room-list-panel__list" role="list">
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
                <li class="room-list-panel__row" data-testid="room-list-row">
                  <div class="room-list-panel__row-main">
                    <div class="room-list-panel__row-title-row">
                      <span class="room-list-panel__row-name">{room.name.clone()}</span>
                      <span
                        class="room-list-panel__row-badge"
                        class:room-list-panel__row-badge--theater=room.room_type == RoomType::Theater
                      >{type_label}</span>
                      <Show when=move || protected>
                        <span
                          class="room-list-panel__row-lock"
                          aria-label=move || t_string!(i18n, room.password_protected)
                          title=move || t_string!(i18n, room.password_protected)
                        >"🔒"</span>
                      </Show>
                    </div>
                    <Show when={
                      let desc = room.description.clone();
                      move || !desc.is_empty()
                    }>
                      <p class="room-list-panel__row-description">{room.description.clone()}</p>
                    </Show>
                    <div class="room-list-panel__row-meta">
                      <span>{
                        let template = t_string!(i18n, room.member_count_format);
                        interpolate_member_count(template, room.member_count.into(), room.max_members.into())
                      }</span>
                    </div>
                  </div>
                  <button
                    type="button"
                    class="btn btn--primary"
                    disabled=is_full
                    on:click=move |_| {
                      if protected {
                        password_target.set(Some(room_for_password.clone()));
                      } else {
                        handle_join.run((room_for_join.clone(), None));
                      }
                    }
                    data-testid="room-list-join"
                  >
                    {if is_full {
                      t_string!(i18n, room.room_full).to_string()
                    } else {
                      t_string!(i18n, room.join).to_string()
                    }}
                  </button>
                </li>
              }
            }
          />
        </ul>
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
    </section>
  }
}
