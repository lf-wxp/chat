//! Sidebar room section — compact room list for the sidebar panel.
//!
//! Shows available rooms with join/create actions in a sidebar-friendly
//! layout. Replaces the full-width `RoomListPanel` that used to live
//! in the main content area.

use std::collections::HashSet;

use icondata as i;
use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use leptos_icons::Icon;
use message::RoomId;
use message::types::{RoomInfo, RoomType};

use crate::components::room::global_modal_context::GlobalRoomModalState;
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

  let modal_state = GlobalRoomModalState::use_global();
  let create_open = modal_state.create_open;
  let password_target = modal_state.password_target;

  // Track which rooms have a `JoinRoom` request in-flight. The signal
  // is cleared once the server answers with `RoomJoined` or an error,
  // which in turn updates `app_state.room_members`. Guarding the click
  // handler against an already-pending request prevents the UI from
  // repeatedly emitting `JoinRoom` (which the server rejects with
  // ROM205 "You are already in a room" once the first attempt has
  // succeeded).
  let joining: RwSignal<HashSet<RoomId>> = RwSignal::new(HashSet::new());

  let visible_rooms = Memo::new(move |_| app_state.rooms.with(|list| list.to_vec()));

  // Reactive set of rooms the current user already belongs to. We
  // derive it from the authoritative `room_members` map rather than
  // maintaining a parallel list, so the badge stays accurate across
  // remote membership updates.
  let joined_rooms = Memo::new(move |_| {
    let me = app_state
      .auth
      .with(|auth| auth.as_ref().map(|a| a.user_id.clone()));
    let Some(me) = me else {
      return HashSet::<RoomId>::new();
    };
    app_state.room_members.with(|map| {
      map
        .iter()
        .filter(|(_, members)| members.iter().any(|m| m.user_id == me))
        .map(|(room_id, _)| room_id.clone())
        .collect()
    })
  });

  let signaling_for_join = signaling.clone();
  let toast_for_join = toast;
  let handle_join: Callback<(RoomInfo, Option<String>)> =
    Callback::new(move |(room, password): (RoomInfo, Option<String>)| {
      // Skip if we are already a member or a join is in-flight.
      if joined_rooms.with(|set| set.contains(&room.room_id))
        || joining.with(|set| set.contains(&room.room_id))
      {
        return;
      }
      joining.update(|set| {
        set.insert(room.room_id.clone());
      });
      if let Err(e) = signaling_for_join.send_join_room(room.room_id.clone(), password) {
        // Roll back the pending flag so the user can retry once the
        // transient failure clears.
        joining.update(|set| {
          set.remove(&room.room_id);
        });
        web_sys::console::warn_1(&format!("[room] Failed to join room: {e}").into());
        toast_for_join.show_error_message_with_key(
          "ROM109",
          "error.rom109",
          t_string!(i18n, error.rom109),
        );
      }
    });

  // When membership changes (either the user joined or was removed),
  // clear the in-flight flag for the affected room so the button can
  // eventually become clickable again.
  Effect::new(move |_| {
    let set = joined_rooms.get();
    joining.update(|pending| {
      pending.retain(|room_id| !set.contains(room_id));
    });
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
            let room_id_for_state = room.room_id.clone();
            let protected = room.is_password_protected();
            let is_full = room.is_full();
            let type_label = match room.room_type {
              RoomType::Chat => t_string!(i18n, room.room_type_chat),
              RoomType::Theater => t_string!(i18n, room.room_type_theater),
            };

            // Reactive per-row state so the button can reflect both
            // the "already joined" and the "pending" cases without
            // reaching into state from the event handler.
            let room_id_for_joined = room_id_for_state.clone();
            let already_joined = Memo::new(move |_| {
              joined_rooms.with(|set| set.contains(&room_id_for_joined))
            });
            let room_id_for_pending = room_id_for_state.clone();
            let is_pending = Memo::new(move |_| {
              joining.with(|set| set.contains(&room_id_for_pending))
            });
            let disabled = Signal::derive(move || {
              is_full || already_joined.get() || is_pending.get()
            });

            // i18n label — falls back to the generic "join" string
            // for full / pending states so the button always has a
            // localised accessible name.
            let label = move || {
              if is_full {
                t_string!(i18n, room.room_full).to_string()
              } else {
                t_string!(i18n, room.join).to_string()
              }
            };
            let label_for_title = label;

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
                  disabled=disabled
                  aria-disabled=disabled
                  aria-label=label
                  title=label_for_title
                  on:click=move |_| {
                    if disabled.get() {
                      return;
                    }
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
    </div>
  }
}
