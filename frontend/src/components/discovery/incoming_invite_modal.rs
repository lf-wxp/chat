//! Incoming connection-invite modal (Req 9.5 — 9.7).
//!
//! Renders the front of the `InviteManager::inbound_signal()` queue
//! with the inviter's avatar, optional note and Accept / Decline
//! buttons. A live countdown displays the remaining seconds before the
//! local 60 s timeout fires (Req 9.8). Multiple back-to-back invites
//! queue up so the user is shown one at a time.
//!
//! Layout / animation are delegated to the shared `ModalWrapper`.
//! Escape and backdrop-click are wired up so dismissal acts as
//! "Decline" (sends `invite_declined`); accidental clicks outside the
//! modal still decline the invite — we deliberately keep this
//! behaviour (rather than disabling backdrop dismissal) because the
//! current product spec treats "ignored = declined" once the 60 s
//! timer expires anyway.

use leptos::prelude::*;
use leptos_i18n::t;
use leptos_use::use_interval_fn;
use wasm_bindgen::JsCast;

use crate::components::room::modal_wrapper::{ModalSize, ModalWrapper};
use crate::error_handler::use_error_toast_manager;
use crate::i18n;
use crate::identicon::generate_identicon_data_uri;
use crate::invite::{IncomingInvite, use_invite_manager};
use crate::signaling::use_signaling_client;
use crate::state::{ConversationId, use_app_state};

/// Hard cap on the rendered note length (defensive truncation). The
/// server already bounds the `note` field during validation, but a
/// misbehaving peer (or a future relaxation of the server limit) could
/// still push through a 50 KB blob, which would blow up the modal
/// height. Truncating here keeps the UI stable even then.
const NOTE_MAX_CHARS: usize = 500;

#[component]
pub fn IncomingInviteModal() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let app_state = use_app_state();
  let invite_mgr = use_invite_manager();
  let signaling = use_signaling_client();
  let toast = use_error_toast_manager();

  // Reactive clock that ticks once per second so the countdown text
  // refreshes without forcing a full re-render of the queue. Reading
  // the clock inside the `remaining_seconds` memo is enough to
  // re-evaluate when the tick fires.
  //
  // `use_interval_fn` registers the timer with the current reactive
  // owner and cancels it automatically on unmount, replacing the
  // hand-rolled `StoredValue<IntervalHandle>` + `on_cleanup` pair
  // used previously (Phase B cleanup — leptos-use migration).
  let now_ms = RwSignal::new(chrono::Utc::now().timestamp_millis());
  use_interval_fn(
    move || {
      now_ms.set(chrono::Utc::now().timestamp_millis());
    },
    1_000_u64,
  );

  let inbound = invite_mgr.inbound_signal();
  let front: Memo<Option<IncomingInvite>> =
    Memo::new(move |_| inbound.with(|q| q.first().cloned()));
  let is_visible = Memo::new(move |_| front.get().is_some());
  let is_open = Signal::derive(move || is_visible.get());

  // Phase C — focus restoration: record the element that owned focus
  // before the modal opened and return focus to it once the modal
  // closes. ModalWrapper handles Escape + backdrop-click + ARIA;
  // we layer focus *return* on top.
  let previous_focus: StoredValue<Option<web_sys::HtmlElement>> = StoredValue::new(None);
  Effect::new(move |prev_visible: Option<bool>| {
    let visible = is_visible.get();
    let was_visible = prev_visible.unwrap_or(false);

    if visible && !was_visible {
      if let Some(document) = web_sys::window().and_then(|w| w.document()) {
        let active = document
          .active_element()
          .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok());
        previous_focus.set_value(active);
      }
    } else if !visible && was_visible {
      let prev = previous_focus.get_value();
      previous_focus.set_value(None);
      if let Some(el) = prev {
        let _ = el.focus();
      }
    }

    visible
  });

  let inviter_label = Memo::new(move |_| {
    front
      .get()
      .map(|i| i.display_name.clone())
      .unwrap_or_default()
  });
  let avatar = Memo::new(move |_| {
    front.get().map_or_else(String::new, |i| {
      let username = if i.display_name.is_empty() {
        i.from.to_string()
      } else {
        i.display_name.clone()
      };
      generate_identicon_data_uri(&username)
    })
  });
  // Phase C — defensive note truncation (see §7 of v3 audit). Limits
  // the rendered note to `NOTE_MAX_CHARS` Unicode scalar values so a
  // pathologically long note can't stretch the modal off-screen. The
  // paired CSS rule (`max-height` + `overflow-y: auto`) provides a
  // second line of defence if a future refactor bypasses this.
  let note = Memo::new(move |_| {
    front.get().and_then(|i| {
      i.note.as_ref().map(|raw| {
        if raw.chars().count() > NOTE_MAX_CHARS {
          let truncated: String = raw.chars().take(NOTE_MAX_CHARS).collect();
          format!("{truncated}…")
        } else {
          raw.clone()
        }
      })
    })
  });
  let remaining_seconds = Memo::new(move |_| {
    front.get().map_or(0_i64, |i| {
      let remaining_ms = i.deadline_ms - now_ms.get();
      remaining_ms.max(0) / 1_000
    })
  });

  let invite_mgr_for_decline = invite_mgr.clone();
  let signaling_for_decline = signaling.clone();
  let invite_mgr_for_accept = invite_mgr.clone();
  let signaling_for_accept = signaling.clone();

  // Decline-via-dismissal (Escape or backdrop click). Mirrors the
  // explicit Decline button so any way of closing the modal
  // consistently sends `invite_declined`.
  let invite_mgr_for_dismiss = invite_mgr.clone();
  let signaling_for_dismiss = signaling.clone();
  let on_close = Callback::new(move |()| {
    let Some(invite) = front.get_untracked() else {
      return;
    };
    invite_mgr_for_dismiss.take_inbound(&invite.from);
    let _ = signaling_for_dismiss.send_invite_declined(&invite.from);
  });

  view! {
    <ModalWrapper
      on_close=on_close
      open=is_open
      size=ModalSize::Medium
      class="incoming-invite-modal"
      labelled_by="incoming-invite-title"
      testid="incoming-invite-modal"
      // Backdrop click is left enabled (= Decline) to match the
      // existing UX where "ignored" maps to "declined". Set to false
      // here if you need a force-answer modal instead.
    >
      <header class="modal-header">
        <h2 id="incoming-invite-title" class="modal-title">
          {t!(i18n, discovery.invite_received_title)}
        </h2>
        // aria-live="off" avoids flooding the screen-reader with a
        // new announcement every second. The countdown is still
        // visible visually; a periodic announcement would be
        // excessively verbose (WCAG 2.1, review §3.4).
        <span class="incoming-invite-modal__countdown" aria-live="off">
          {move || format!("{}s", remaining_seconds.get())}
        </span>
      </header>

      <div class="modal-body incoming-invite-modal__body">
        <img
          class="incoming-invite-modal__avatar"
          src=move || avatar.get()
          alt=""
          width="72"
          height="72"
        />
        <p class="incoming-invite-modal__inviter">{move || inviter_label.get()}</p>
        <Show when=move || note.get().is_some()>
          <blockquote class="incoming-invite-modal__note">
            {move || note.get().unwrap_or_default()}
          </blockquote>
        </Show>
      </div>

      <footer class="modal-footer">
        <button
          type="button"
          class="btn btn--ghost"
          on:click={
            let invite_mgr = invite_mgr_for_decline.clone();
            let signaling = signaling_for_decline.clone();
            move |_| {
              let Some(invite) = front.get_untracked() else {
                return;
              };
              invite_mgr.take_inbound(&invite.from);
              if let Err(e) = signaling.send_invite_declined(&invite.from) {
                toast.show_error_message_with_key(
                  "SIG001",
                  "discovery.invite_failed",
                  &format!("Failed to send decline: {e}"),
                );
              }
            }
          }
          data-testid="invite-decline"
        >
          {t!(i18n, discovery.decline)}
        </button>
        <button
          type="button"
          class="btn btn--primary"
          on:click={
            let invite_mgr = invite_mgr_for_accept.clone();
            let signaling = signaling_for_accept.clone();
            move |_| {
              let Some(invite) = front.get_untracked() else {
                return;
              };
              invite_mgr.take_inbound(&invite.from);
              if let Err(e) = signaling.send_invite_accepted(&invite.from) {
                toast.show_error_message_with_key(
                  "SIG001",
                  "discovery.invite_failed",
                  &format!("Failed to accept invite: {e}"),
                );
                return;
              }
              let conv = ConversationId::Direct(invite.from.clone());
              let display = invite.display_name.clone();
              app_state.conversations.update(|list| {
                if !list.iter().any(|c| c.id == conv) {
                  list.push(crate::state::Conversation {
                    id: conv.clone(),
                    display_name: display,
                    last_message: None,
                    last_message_ts: Some(chrono::Utc::now().timestamp_millis()),
                    unread_count: 0,
                    pinned: false,
                    pinned_ts: None,
                    muted: false,
                    archived: false,
                    conversation_type: crate::state::ConversationType::Direct,
                  });
                }
              });
              app_state.active_conversation.set(Some(conv));
            }
          }
          data-testid="invite-accept"
        >
          {t!(i18n, discovery.accept)}
        </button>
      </footer>
    </ModalWrapper>
  }
}
