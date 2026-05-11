//! Start-call button for the chat view header.
//!
//! Renders a single round button that, when clicked, kicks off
//! [`CallManager::initiate_call`] with the active room id and
//! `MediaType::Video`. The button only appears for `Room`
//! conversations because the server's `handle_call_invite` requires a
//! real `RoomId` — direct (peer-to-peer) calls would need a synthetic
//! room or a separate signaling path that does not yet exist.
//!
//! Wave P0-5 of the E2E coverage plan added this so an A/V call can
//! be triggered through the UI rather than only via test-only hooks.
//! Without it the call subsystem (manager, modal, view, controls)
//! existed but had no user-facing entry point.

use icondata as i;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_i18n::t_string;
use leptos_icons::Icon;
use message::types::MediaType;

use crate::call::{CallState, use_call_manager, use_call_signals};
use crate::i18n;
use crate::state::ConversationId;

/// Floating "Start video call" button rendered inside the chat view.
///
/// * `conv` — the current active conversation. The button is only
///   visible for [`ConversationId::Room`] entries.
#[component]
pub fn CallStartButton(conv: Signal<Option<ConversationId>>) -> impl IntoView {
  let i18n = i18n::use_i18n();
  let call_manager = use_call_manager();
  let call_signals = use_call_signals();

  // The button is only meaningful for room conversations and when the
  // call manager is currently idle. If a call is already ringing /
  // active / inviting, hide the button so the user uses the dedicated
  // call view's controls instead.
  let visible = Memo::new(move |_| {
    let is_room = matches!(conv.get(), Some(ConversationId::Room(_)));
    let is_idle = matches!(call_signals.call_state.get(), CallState::Idle);
    is_room && is_idle
  });

  // Wrap the click action in a `Callback` so it stays `Fn` across
  // every `<Show>` rebuild (the body of `<Show>` is a `ChildrenFn`
  // which Leptos re-evaluates on every visibility change).
  let on_click: Callback<()> = Callback::new(move |()| {
    let Some(ConversationId::Room(room_id)) = conv.get_untracked() else {
      return;
    };
    let manager = call_manager.clone();
    spawn_local(async move {
      if let Err(e) = manager.initiate_call(room_id, MediaType::Video).await {
        web_sys::console::warn_1(&format!("[call] initiate failed: {e}").into());
      }
    });
  });

  view! {
    <Show when=move || visible.get() fallback=|| ()>
      <button
        type="button"
        class="chat-view__call-start-btn"
        on:click=move |_| on_click.run(())
        aria-label=move || t_string!(i18n, call.video_call)
        title=move || t_string!(i18n, call.video_call)
        data-testid="call-start-btn"
      >
        <Icon icon=i::LuVideo />
      </button>
    </Show>
  }
}
