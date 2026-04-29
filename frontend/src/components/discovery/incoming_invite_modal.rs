//! Incoming connection-invite modal (Req 9.5 — 9.7).
//!
//! Renders the front of the `InviteManager::inbound_signal()` queue
//! with the inviter's avatar, optional note and Accept / Decline
//! buttons. A live countdown displays the remaining seconds before the
//! local 60 s timeout fires (Req 9.8). Multiple back-to-back invites
//! queue up so the user is shown one at a time.

use leptos::ev::keydown;
use leptos::prelude::*;
use leptos_i18n::t;
use leptos_use::{use_document, use_event_listener, use_interval_fn};
use wasm_bindgen::JsCast;

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

  // §3.1 P1 fix — listen for `Escape` on the document so dismissal
  // works regardless of where focus lives. `use_event_listener`
  // auto-removes the listener on cleanup; no manual `StoredValue`
  // bookkeeping is needed anymore.
  let keydown_mgr = invite_mgr.clone();
  let keydown_signaling = signaling.clone();
  let _ = use_event_listener(
    use_document(),
    keydown,
    move |ev: web_sys::KeyboardEvent| {
      if ev.key() != "Escape" {
        return;
      }
      let Some(invite) = front.get_untracked() else {
        return;
      };
      keydown_mgr.take_inbound(&invite.from);
      let _ = keydown_signaling.send_invite_declined(&invite.from);
    },
  );

  // Phase C — focus restoration: record the element that owned focus
  // before the modal opened and return focus to it once the modal
  // closes. A full Tab-cycle focus trap is deferred to task 24;
  // restoring focus already closes the main a11y regression flagged
  // in §5 ("focus restoration after modal close ❌").
  let previous_focus: StoredValue<Option<web_sys::HtmlElement>> = StoredValue::new(None);
  let modal_ref: NodeRef<leptos::html::Div> = NodeRef::new();
  Effect::new(move |prev_visible: Option<bool>| {
    let visible = is_visible.get();
    let was_visible = prev_visible.unwrap_or(false);

    if visible && !was_visible {
      // Modal just opened — capture the current focus target and move
      // focus onto the modal container so Tab starts cycling inside.
      if let Some(document) = web_sys::window().and_then(|w| w.document()) {
        let active = document
          .active_element()
          .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok());
        previous_focus.set_value(active);
      }
      if let Some(el) = modal_ref.get() {
        let div_el: &web_sys::HtmlDivElement = el.as_ref();
        let _ = div_el.focus();
      }
    } else if !visible && was_visible {
      // Modal just closed — restore focus to the triggering element.
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

  view! {
    <Show when=move || is_visible.get()>
      <div
        class="modal-backdrop modal-backdrop-visible"
        role="presentation"
        data-testid="invite-backdrop"
      >
        <div
          class="modal incoming-invite-modal"
          node_ref=modal_ref
          role="dialog"
          aria-modal="true"
          aria-labelledby="incoming-invite-title"
          tabindex="-1"
          data-testid="incoming-invite-modal"
        >
          <header class="modal-header">
            <h2 id="incoming-invite-title" class="modal-title">
              {t!(i18n, discovery.invite_received_title)}
            </h2>
            <span class="incoming-invite-modal__countdown" aria-live="polite">
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
        </div>
      </div>
    </Show>
  }
}
