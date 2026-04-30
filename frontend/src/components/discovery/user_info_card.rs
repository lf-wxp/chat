//! User info card modal.
//!
//! Rendered as an overlay when the user clicks a row in the online
//! users panel for a user that is **not yet connected** (Req 9.3).
//! Provides the inviter's nickname, signature, status and an optional
//! "Add a note" textarea before sending the invitation. Also exposes
//! the Block / Unblock toggle so users can manage the local blacklist
//! directly from this card (Req 9.15).

use leptos::ev::keydown;
use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use leptos_use::{use_document, use_event_listener};
use message::UserId;
use message::types::UserStatus;
use wasm_bindgen::JsCast;

use crate::blacklist::use_blacklist_state;
use crate::error_handler::use_error_toast_manager;
use crate::i18n;
use crate::identicon::generate_identicon_data_uri;
use crate::invite::{InviteStatus, use_invite_manager};
use crate::signaling::use_signaling_client;
use crate::state::use_app_state;
use crate::webrtc::try_use_webrtc_manager;
use icondata as i;
use leptos_icons::Icon;

/// User info card component. Renders nothing while `target` is `None`.
#[component]
pub fn UserInfoCard(
  /// Reactive selection — `Some(user_id)` opens the card; setting to
  /// `None` closes it. Updated by parent on close.
  target: RwSignal<Option<UserId>>,
) -> impl IntoView {
  let i18n = i18n::use_i18n();
  let app_state = use_app_state();
  let blacklist = use_blacklist_state();
  let invite_mgr = use_invite_manager();
  let signaling = use_signaling_client();
  let toast = use_error_toast_manager();

  let note = RwSignal::new(String::new());

  // Reset the note input every time the card opens for a new user.
  Effect::new(move |_| {
    if target.get().is_some() {
      note.set(String::new());
    }
  });

  // §3.1 P1 fix — Escape dismiss at document level via leptos-use so
  // the handler fires regardless of current focus and is removed
  // automatically when the component unmounts.
  let _ = use_event_listener(
    use_document(),
    keydown,
    move |ev: web_sys::KeyboardEvent| {
      if ev.key() == "Escape" && target.get_untracked().is_some() {
        target.set(None);
      }
    },
  );

  // Phase C — focus restoration. Capture the element that owned focus
  // when the card opens and return focus to it when the card closes.
  // A full Tab-cycle focus trap is deferred to task 24.
  let previous_focus: StoredValue<Option<web_sys::HtmlElement>> = StoredValue::new(None);
  let modal_ref: NodeRef<leptos::html::Div> = NodeRef::new();
  Effect::new(move |prev_open: Option<bool>| {
    let open = target.get().is_some();
    let was_open = prev_open.unwrap_or(false);

    if open && !was_open {
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
    } else if !open && was_open {
      let prev = previous_focus.get_value();
      previous_focus.set_value(None);
      if let Some(el) = prev {
        let _ = el.focus();
      }
    }

    open
  });

  let online_users = app_state.online_users;
  let selected_user = Memo::new(move |_| {
    let id = target.get()?;
    online_users.with(|list| list.iter().find(|u| u.user_id == id).cloned())
  });

  let display = Memo::new(move |_| {
    selected_user.get().map_or_else(String::new, |u| {
      if u.nickname.is_empty() {
        u.username
      } else {
        u.nickname
      }
    })
  });

  let avatar = Memo::new(move |_| {
    selected_user.get().map_or_else(String::new, |u| {
      u.avatar_url
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| generate_identicon_data_uri(&u.username))
    })
  });

  let signature = Memo::new(move |_| selected_user.get().map(|u| u.bio).unwrap_or_default());
  let status = Memo::new(move |_| {
    selected_user
      .get()
      .map_or(UserStatus::Offline, |u| u.status)
  });
  let blocked = {
    let bl = blacklist.clone();
    Memo::new(move |_| target.get().map(|id| bl.is_blocked(&id)).unwrap_or(false))
  };
  let invite_status = {
    let mgr = invite_mgr.clone();
    Memo::new(move |_| target.get().and_then(|id| mgr.outbound_status(&id)))
  };
  let pending = Memo::new(move |_| matches!(invite_status.get(), Some(InviteStatus::Pending)));
  let connecting =
    Memo::new(move |_| matches!(invite_status.get(), Some(InviteStatus::Connecting)));

  let close = move || target.set(None);

  // Snapshot per-handler clones outside the `view!` so each rebuild of
  // the `Show` children gets fresh `Fn` closures (Leptos' ChildrenFn
  // contract). `signaling`, `invite_mgr`, and `blacklist` are `Clone`
  // but not `Copy`, so we materialise one clone per click handler that
  // needs them.
  let signaling_send = signaling.clone();
  let invite_mgr_send = invite_mgr.clone();
  let invite_mgr_block = invite_mgr.clone();
  let blacklist_for_block = blacklist.clone();

  view! {
    <Show when=move || target.get().is_some()>
      <div
        class="modal-backdrop modal-backdrop-visible"
        role="presentation"
        on:click=move |_| close()
        data-testid="user-info-backdrop"
      >
        <div
          class="modal user-info-card"
          node_ref=modal_ref
          role="dialog"
          aria-modal="true"
          aria-labelledby="user-info-card-title"
          on:click=move |ev| ev.stop_propagation()
          tabindex="-1"
          data-testid="user-info-card"
        >
          <header class="modal-header">
            <h2 id="user-info-card-title" class="modal-title">
              {move || display.get()}
            </h2>
            <button
              type="button"
              class="modal-close"
              aria-label=move || t_string!(i18n, common.close)
              on:click=move |_| close()
            ><Icon icon=i::LuX /></button>
          </header>

          <div class="modal-body user-info-card__body">
            <img
              class="user-info-card__avatar"
              src=move || avatar.get()
              alt=""
              width="96"
              height="96"
            />
            <p class="user-info-card__status">
              {move || format!("{:?}", status.get())}
            </p>
            <Show when=move || !signature.get().is_empty()>
              <p class="user-info-card__signature">{move || signature.get()}</p>
            </Show>

            <Show when=move || connecting.get()>
              <p
                class="user-info-card__connecting"
                role="status"
                aria-live="polite"
                data-testid="user-info-connecting"
              >
                {t!(i18n, discovery.connecting_please_wait)}
              </p>
            </Show>

            <label class="user-info-card__note-label" for="user-info-note">
              {t!(i18n, discovery.note_label)}
            </label>
            <textarea
              id="user-info-note"
              class="user-info-card__note"
              maxlength="200"
              placeholder=move || t_string!(i18n, discovery.note_placeholder)
              prop:value=move || note.get()
              on:input=move |ev| note.set(event_target_value(&ev))
              disabled=move || blocked.get() || pending.get() || connecting.get()
            ></textarea>
          </div>

          <footer class="modal-footer user-info-card__actions">
            <button
              type="button"
              class=move || {
                if blocked.get() {
                  "btn btn--ghost user-info-card__block is-blocked"
                } else {
                  "btn btn--ghost user-info-card__block"
                }
              }
              on:click={
                let invite_mgr = invite_mgr_block.clone();
                let blacklist = blacklist_for_block.clone();
                move |_| {
                  // P2-3.2 fix: read `target` untracked inside the
                  // event handler so the closure does not subscribe
                  // to it (consistent with `multi_invite_panel.rs` /
                  // `online_users_panel.rs`).
                  let Some(target_id) = target.get_untracked() else {
                    return;
                  };
                  if blacklist.is_blocked(&target_id) {
                    blacklist.unblock(&target_id);
                  } else {
                    let display_name = display.get_untracked();
                    blacklist.block(target_id.clone(), display_name);
                    if let Some(mgr) = try_use_webrtc_manager() {
                      mgr.close_connection(&target_id);
                    }
                    invite_mgr.cancel_outbound(&target_id);
                    close();
                  }
                }
              }
              data-testid="user-info-block"
            >
              {move || {
                if blocked.get() {
                  t_string!(i18n, discovery.unblock)
                } else {
                  t_string!(i18n, discovery.block)
                }
              }}
            </button>
            <button
              type="button"
              class="btn btn--primary user-info-card__invite"
              prop:disabled=move || blocked.get() || pending.get() || connecting.get()
              title=move || {
                if blocked.get() {
                  t_string!(i18n, discovery.blocked_tooltip)
                } else if connecting.get() {
                  t_string!(i18n, discovery.connecting_please_wait)
                } else if pending.get() {
                  t_string!(i18n, discovery.invite_pending_tooltip)
                } else {
                  t_string!(i18n, discovery.send_invite)
                }
              }
              on:click={
                let signaling = signaling_send.clone();
                let invite_mgr = invite_mgr_send.clone();
                move |_| {
                  // P2-3.2 fix: consistent with the block handler,
                  // read the signals untracked inside the callback.
                  let Some(target_id) = target.get_untracked() else {
                    return;
                  };
                  let display_name = display.get_untracked();
                  let note_text = note.get_untracked();
                  let note_payload = if note_text.trim().is_empty() {
                    None
                  } else {
                    Some(note_text.trim().to_string())
                  };
                  if !invite_mgr.track_outbound(target_id.clone(), display_name) {
                    web_sys::console::warn_1(
                      &"[invite] duplicate invite suppressed".into(),
                    );
                    return;
                  }
                  if let Err(e) =
                    signaling.send_connection_invite(&target_id, note_payload)
                  {
                    invite_mgr.cancel_outbound(&target_id);
                    toast.show_error_message_with_key(
                      "SIG001",
                      "discovery.invite_failed",
                      &format!("Failed to send invitation: {e}"),
                    );
                  }
                }
              }
              data-testid="user-info-invite"
            >
              {move || {
                if connecting.get() {
                  t_string!(i18n, discovery.connecting_please_wait)
                } else if pending.get() {
                  t_string!(i18n, discovery.inviting)
                } else {
                  t_string!(i18n, discovery.send_invite)
                }
              }}
            </button>
          </footer>
        </div>
      </div>
    </Show>
  }
}
