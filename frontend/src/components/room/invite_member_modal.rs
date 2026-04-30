//! Invite-to-room modal (Req 4.3 — Owner-facing send UI).
//!
//! Allows the room owner to select one or more online users who are not
//! yet in the room and send them a `RoomInvite` signalling message
//! with an optional note.

use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use message::UserId;
use message::types::RoomInfo;

use crate::components::room::modal_wrapper::{ModalSize, ModalWrapper};
use crate::error_handler::use_error_toast_manager;
use crate::i18n;
use crate::signaling::use_signaling_client;
use crate::state::use_app_state;

/// Maximum number of recipients per invitation batch to prevent abuse.
const INVITE_MAX: usize = 5;

/// Invite-to-room modal.
#[component]
pub fn InviteMemberModal(
  /// Room whose members should be invited.
  #[prop(into)]
  room: Signal<RoomInfo>,
  /// Controls visibility. The parent flips this signal; the modal
  /// sets it back to `false` on close / send.
  #[prop(into)]
  open: RwSignal<bool>,
) -> impl IntoView {
  let i18n = i18n::use_i18n();
  let app_state = use_app_state();
  let signaling = use_signaling_client();
  let toast = use_error_toast_manager();

  let query = RwSignal::new(String::new());
  let note = RwSignal::new(String::new());
  let selected = RwSignal::new(Vec::<UserId>::new());

  // Online users who are NOT already in this room.
  let eligible_users = Memo::new(move |_| {
    let rid = room.with(|r| r.room_id.clone());
    let existing: Vec<UserId> = app_state.room_members.with(|map| {
      map
        .get(&rid)
        .map(|list| list.iter().map(|m| m.user_id.clone()).collect())
        .unwrap_or_default()
    });
    app_state.online_users.with(|users| {
      users
        .iter()
        .filter(|u| !existing.contains(&u.user_id))
        .cloned()
        .collect::<Vec<_>>()
    })
  });

  // Filtered by search query.
  let filtered_users = Memo::new(move |_| {
    let q = query.get().trim().to_lowercase();
    let users = eligible_users.get();
    if q.is_empty() {
      return users.to_vec();
    }
    users
      .into_iter()
      .filter(|u| {
        u.nickname.to_lowercase().contains(&q)
          || u.username.to_lowercase().contains(&q)
          || u.user_id.to_string().to_lowercase().contains(&q)
      })
      .collect()
  });

  let is_selected = move |uid: &UserId| -> bool { selected.with(|s| s.contains(uid)) };

  let toggle_select = move |uid: UserId| {
    selected.update(|s| {
      if let Some(pos) = s.iter().position(|u| u == &uid) {
        s.remove(pos);
      } else if s.len() < INVITE_MAX {
        s.push(uid);
      }
    });
  };

  let on_close = Callback::new(move |()| {
    open.set(false);
    query.set(String::new());
    note.set(String::new());
    selected.set(Vec::new());
  });

  let on_send = Callback::new(move |()| {
    let targets = selected.get_untracked();
    if targets.is_empty() {
      return;
    }
    let rid = room.with(|r| r.room_id.clone());
    let note_text = note.get_untracked();
    let mut errors = 0_usize;
    for target in &targets {
      if signaling
        .send_room_invite(rid.clone(), target.clone(), note_text.clone())
        .is_err()
      {
        errors += 1;
      }
    }
    if errors > 0 {
      let msg = t_string!(i18n, room.invite_send_failed);
      let msg = msg.replace("{count}", &errors.to_string());
      toast.show_error_message_with_key("ROM2402", "room.invite_send_failed", &msg);
    } else {
      toast.show_info_message_with_key(
        "ROM2401",
        "room.invite_sent",
        t_string!(i18n, room.invite_sent),
      );
    }
    open.set(false);
    query.set(String::new());
    note.set(String::new());
    selected.set(Vec::new());
  });

  let room_name = Memo::new(move |_| room.with(|r| r.name.clone()));

  view! {
    <Show when=move || open.get()>
      <ModalWrapper
        on_close=on_close
        size=ModalSize::Small
        class="invite-member-modal"
        labelled_by="invite-member-title"
        testid="invite-member-modal"
      >
        <header class="modal-header">
          <h2 id="invite-member-title" class="modal-title">
            {t!(i18n, room.invite_to_room)}
          </h2>
        </header>
        <div class="modal-body">
          <div class="invite-member-modal__room-name">
            {move || room_name.get()}
          </div>
          <div class="invite-member-modal__search">
            <input
              type="search"
              class="input"
              placeholder=move || t_string!(i18n, common.search)
              prop:value=move || query.get()
              on:input=move |ev| query.set(event_target_value(&ev))
              aria-label=move || t_string!(i18n, common.search)
              data-testid="invite-member-search"
            />
          </div>
          <Show when=move || !filtered_users.get().is_empty()>
            <ul class="invite-member-modal__list" role="listbox"
              aria-label=move || t_string!(i18n, room.invite_to_room)
              data-testid="invite-member-list"
            >
              <For
                each=move || filtered_users.get()
                key=|u| u.user_id.clone()
                children=move |user| {
                  let uid = user.user_id.clone();
                  let display_name = if user.nickname.is_empty() {
                    user.username.clone()
                  } else {
                    user.nickname.clone()
                  };
                  let uid_for_aria_selected = uid.clone();
                  let uid_for_checked = uid.clone();
                  let uid_for_toggle = uid.clone();
                  let display_for_aria = display_name.clone();
                  view! {
                    <li class="invite-member-modal__item" role="option"
                      aria-selected=move || is_selected(&uid_for_aria_selected)
                    >
                      <label class="invite-member-modal__label">
                        <input
                          type="checkbox"
                          checked=move || is_selected(&uid_for_checked)
                          on:change=move |_| toggle_select(uid_for_toggle.clone())
                          aria-label=move || display_for_aria.clone()
                          data-testid="invite-member-checkbox"
                        />
                        <span class="invite-member-modal__name">
                          {display_name}
                        </span>
                      </label>
                    </li>
                  }
                }
              />
            </ul>
          </Show>
          <Show when=move || filtered_users.get().is_empty() && !eligible_users.get().is_empty()>
            <p class="invite-member-modal__empty" data-testid="invite-member-no-results">
              {move || {
                let q = query.get();
                let tpl = t_string!(i18n, room.no_results_for);
                tpl.replace("{query}", &q)
              }}
            </p>
          </Show>
          <Show when=move || eligible_users.get().is_empty()>
            <p class="invite-member-modal__empty" data-testid="invite-member-all-joined">
              {t!(i18n, room.invite_all_joined)}
            </p>
          </Show>
          <div class="invite-member-modal__note">
            <label class="invite-member-modal__note-label" for="invite-member-note">
              {t!(i18n, room.invite_note_label)}
            </label>
            <input
              id="invite-member-note"
              type="text"
              class="input"
              placeholder=move || t_string!(i18n, room.invite_note_placeholder)
              prop:value=move || note.get()
              on:input=move |ev| note.set(event_target_value(&ev))
              maxlength="100"
              data-testid="invite-member-note"
            />
          </div>
          <div class="invite-member-modal__selection">
            <span class="invite-member-modal__count">
              {move || format!("{} / {INVITE_MAX}", selected.with(Vec::len))}
            </span>
          </div>
        </div>
        <footer class="modal-footer">
          <button
            type="button"
            class="btn btn--ghost"
            on:click=move |_| on_close.run(())
            data-testid="invite-member-cancel"
          >
            {t!(i18n, common.cancel)}
          </button>
          <button
            type="button"
            class="btn btn--primary"
            disabled=move || selected.with(Vec::is_empty)
            on:click=move |_| on_send.run(())
            data-testid="invite-member-send"
          >
            {t!(i18n, room.invite_send)}
          </button>
        </footer>
      </ModalWrapper>
    </Show>
  }
}
