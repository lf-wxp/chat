//! Incoming room invite modal (Req 4.4 — Sprint 5.4).
//!
//! Reads `app_state.pending_room_invite` and renders a small modal
//! when an invite arrives. The user can accept (auto-joins via
//! `JoinRoom`) or decline. If the room is password-protected, a
//! password prompt is shown before joining.

use leptos::prelude::*;
use leptos_i18n::{t, t_string};

use crate::components::room::modal_wrapper::{ModalSize, ModalWrapper};
use crate::components::room::password_prompt_modal::{PasswordPromptModal, PasswordPromptMode};
use crate::components::room::utils::interpolate_from_and_room;
use crate::error_handler::use_error_toast_manager;
use crate::i18n;
use crate::signaling::use_signaling_client;
use crate::state::use_app_state;

/// Incoming room invite modal.
#[component]
pub fn IncomingRoomInviteModal() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let app_state = use_app_state();
  let signaling = use_signaling_client();
  let toast = use_error_toast_manager();
  let signaling_store = StoredValue::new(signaling);

  let invite = app_state.pending_room_invite;
  // Whether a password prompt is open (for password-protected rooms).
  let password_open = RwSignal::new(false);

  let from_name = Memo::new(move |_| {
    invite
      .with(|i| {
        i.as_ref().map(|inv| {
          let from_id = &inv.from;
          // Try to resolve the inviter's display name from online_users
          // or room_members; fall back to the raw UserId string.
          app_state.online_users.with(|users| {
            users
              .iter()
              .find(|u| &u.user_id == from_id)
              .map(|u| {
                if u.nickname.is_empty() {
                  u.user_id.to_string()
                } else {
                  u.nickname.clone()
                }
              })
              .unwrap_or_else(|| from_id.to_string())
          })
        })
      })
      .unwrap_or_default()
  });
  let room_name = Memo::new(move |_| {
    invite
      .with(|i| {
        i.as_ref().map(|i| {
          let rid = i.room_id.clone();
          app_state.rooms.with(|rooms| {
            rooms
              .iter()
              .find(|r| r.room_id == rid)
              .map(|r| r.name.clone())
              .unwrap_or_else(|| rid.to_string())
          })
        })
      })
      .unwrap_or_default()
  });

  // Whether the invited room is password-protected.
  let is_protected = Memo::new(move |_| {
    invite
      .with(|i| {
        i.as_ref().map(|i| {
          let rid = i.room_id.clone();
          app_state.rooms.with(|rooms| {
            rooms
              .iter()
              .find(|r| r.room_id == rid)
              .map(|r| r.is_password_protected())
          })
        })
      })
      .flatten()
      .unwrap_or(false)
  });

  let on_close = Callback::new(move |()| {
    invite.set(None);
  });

  // Actually join the room after accepting the invite. Uses a closure
  // instead of a standalone function so the i18n context can be
  // captured without naming the generated Locale type.
  let do_join = move |password: Option<String>| {
    let Some(inv) = invite.get_untracked() else {
      return;
    };
    let inviter = inv.from.clone();
    let room_id = inv.room_id.clone();
    signaling_store.with_value(|s| {
      let _ = s.send_room_invite_response(room_id.clone(), inviter, true);
      let _ = s.send_join_room(room_id, password);
    });
    toast.show_info_message_with_key(
      "ROM2401",
      "room.invite_accepted",
      t_string!(i18n, room.invite_accepted),
    );
    invite.set(None);
  };

  let on_accept = move |_| {
    if is_protected.get() {
      // Show password prompt before joining.
      password_open.set(true);
    } else {
      do_join(None);
    }
  };

  let on_decline = move |_| {
    let Some(inv) = invite.get_untracked() else {
      return;
    };
    let inviter = inv.from.clone();
    let room_id = inv.room_id.clone();
    signaling_store.with_value(|s| {
      let _ = s.send_room_invite_response(room_id, inviter, false);
    });
    toast.show_info_message_with_key(
      "ROM2402",
      "room.invite_declined",
      t_string!(i18n, room.invite_declined),
    );
    invite.set(None);
  };

  let on_password_submit = Callback::new(move |pwd: String| {
    password_open.set(false);
    do_join(Some(pwd));
  });
  let on_password_cancel = Callback::new(move |()| {
    password_open.set(false);
  });

  view! {
    <Show when=move || invite.with(Option::is_some)>
      <ModalWrapper
        on_close=on_close
        size=ModalSize::Small
        class="incoming-room-invite-modal"
        labelled_by="incoming-room-invite-title"
        testid="incoming-room-invite-modal"
      >
        <header class="modal-header">
          <h2 id="incoming-room-invite-title" class="modal-title">
            {move || {
              let template = t_string!(i18n, room.invite_room_title);
              interpolate_from_and_room(template, &from_name.get(), "")
            }}
          </h2>
        </header>
        <div class="modal-body">
          <p>
            {move || {
              let template = t_string!(i18n, room.invite_room_body);
              interpolate_from_and_room(template, &from_name.get(), &room_name.get())
            }}
          </p>
        </div>
        <footer class="modal-footer">
          <button
            type="button"
            class="btn btn--ghost"
            on:click=on_decline
            data-testid="incoming-room-invite-decline"
          >
            {t!(i18n, room.invite_decline)}
          </button>
          <button
            type="button"
            class="btn btn--primary"
            on:click=on_accept
            data-testid="incoming-room-invite-accept"
          >
            {t!(i18n, room.invite_accept)}
          </button>
        </footer>
      </ModalWrapper>
    </Show>

    <Show when=move || password_open.get()>
      <PasswordPromptModal
        title=Signal::derive(move || t_string!(i18n, room.password).to_string())
        mode=PasswordPromptMode::JoinExisting
        on_submit=on_password_submit
        on_cancel=on_password_cancel
      />
    </Show>
  }
}
