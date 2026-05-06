//! Theater chat panel (Req 12.6).
//!
//! Sits alongside the video surface and renders the plain-text chat
//! feed for theater participants. Three responsibilities:
//!
//! 1. **Display** — subscribe to `state.chat_messages`, render each
//!    entry as a [`TheaterChatBubble`], auto-scroll to the latest
//!    message whenever the list grows.
//! 2. **Authoring** — a small composer broadcasts `ChatText` via the
//!    DataChannel and echoes a local copy to keep the UI responsive.
//! 3. **Unread tracking** — whenever the panel mounts / becomes
//!    active it calls [`TheaterState::mark_chat_read`] so the unread
//!    badge clears.
//!
//! Note: the sanitisation policy for user messages lives in the
//! chat layer (`message::error::validation::validate_message`); the
//! panel only performs the cheap length guard before reaching it.

use std::collections::VecDeque;

use js_sys::Date;
use leptos::html;
use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use message::datachannel::{DataChannelMessage, TheaterChatText};
use message::error::validation::validate_message;

use crate::components::theater::TheaterChatBubble;
use crate::i18n;
use crate::state::use_app_state;
use crate::theater::{TheaterChatMessage, use_theater_state};
use crate::webrtc::try_use_webrtc_manager;

/// Maximum payload length. Longer messages are rejected client-side
/// with an inline error so we do not waste a round-trip.
const MAX_CHAT_LEN: usize = 500;

/// How often the ambient "now" signal ticks so relative-time labels
/// stay fresh without a per-bubble timer. 15s strikes a sensible
/// balance between responsiveness and needless re-renders.
const RELATIVE_TIME_TICK_MS: u32 = 15_000;

/// Theater chat panel.
#[component]
#[allow(clippy::needless_pass_by_value)]
pub fn TheaterChatPanel() -> impl IntoView {
  let state = use_theater_state();
  let app = use_app_state();
  let i18n = i18n::use_i18n();

  let composer = RwSignal::<String>::new(String::new());
  let error = RwSignal::<Option<String>>::new(None);
  let now_ms = RwSignal::<u64>::new(Date::now() as u64);

  let list_ref: NodeRef<html::Ul> = NodeRef::new();

  // Ambient "now" tick so every bubble updates its relative time
  // without needing an individual timer.
  let tick = set_interval_with_handle(
    move || {
      now_ms.set(Date::now() as u64);
    },
    std::time::Duration::from_millis(u64::from(RELATIVE_TIME_TICK_MS)),
  );
  if let Ok(handle) = tick {
    on_cleanup(move || handle.clear());
  }

  // Clear the unread counter whenever the panel mounts / remounts.
  Effect::new(move |_| {
    state.mark_chat_read();
  });

  // Auto-scroll to the bottom on every new message. `with` returns
  // the length so we subscribe specifically to growth, avoiding a
  // rerun whenever a bubble's relative-time label flips.
  let message_count = Signal::derive(move || state.chat_messages.with(VecDeque::len));
  Effect::new(move |_| {
    let _ = message_count.get();
    if let Some(list) = list_ref.get() {
      list.set_scroll_top(list.scroll_height());
    }
    // Also clear the unread counter since we just surfaced the new
    // message inside the currently-visible panel.
    state.mark_chat_read();
  });

  let handle_submit = move |ev: leptos::ev::SubmitEvent| {
    ev.prevent_default();
    if !state.can_speak() {
      error.set(Some(t_string!(i18n, theater.chat_muted).to_string()));
      return;
    }
    let raw = composer.get();
    let trimmed = raw.trim();
    if trimmed.is_empty() {
      return;
    }
    if trimmed.chars().count() > MAX_CHAT_LEN {
      error.set(Some(t_string!(i18n, theater.chat_too_long).to_string()));
      return;
    }
    if validate_message(trimmed).is_err() {
      error.set(Some(t_string!(i18n, theater.chat_too_long).to_string()));
      return;
    }

    let now = Date::now() as u64;
    let timestamp_nanos = now.saturating_mul(1_000_000);
    let Some(room_id) = state.room_id.get_untracked() else {
      return;
    };

    let (sender_id, sender_name) = app.auth.with_untracked(|a| {
      a.as_ref().map_or_else(
        || (message::UserId::default(), String::from("?")),
        |auth| (auth.user_id.clone(), auth.nickname.clone()),
      )
    });
    let payload = TheaterChatText {
      room_id,
      sender_id: sender_id.clone(),
      content: trimmed.to_string(),
      timestamp_nanos,
    };
    state.push_chat_message(TheaterChatMessage {
      id: state.next_chat_message_id(),
      sender_id,
      sender_name,
      content: trimmed.to_string(),
      sent_at_ms: now,
      is_self: true,
    });

    if let Some(manager) = try_use_webrtc_manager() {
      manager.broadcast_data_channel_message(&DataChannelMessage::TheaterChatText(payload));
    }

    composer.set(String::new());
    error.set(None);
  };

  let is_muted = move || !state.can_speak();

  view! {
    <section
      class="theater-chat-panel"
      aria-label=move || t_string!(i18n, theater.chat_title)
      data-testid="theater-chat-panel"
    >
      <header class="theater-chat-panel__header">
        <h3 class="theater-chat-panel__title">{t!(i18n, theater.chat_title)}</h3>
      </header>

      <ul
        node_ref=list_ref
        class="theater-chat-panel__list"
        role="list"
        aria-live="polite"
      >
        <Show
          when=move || state.chat_messages.with(|m| !m.is_empty())
          fallback=move || view! {
            <li class="theater-chat-panel__empty" role="note">
              {t!(i18n, theater.chat_empty)}
            </li>
          }
        >
          <For
            each=move || state.chat_messages.get()
            key=|msg: &TheaterChatMessage| msg.id
            children=move |msg: TheaterChatMessage| {
              view! { <TheaterChatBubble message=msg now_ms=now_ms /> }
            }
          />
        </Show>
      </ul>

      <form class="theater-chat-panel__composer" on:submit=handle_submit>
        <Show when=is_muted>
          <p class="theater-chat-panel__muted" role="note">
            {t!(i18n, theater.chat_muted)}
          </p>
        </Show>
        <div class="theater-chat-panel__composer-row">
          <input
            class="input theater-chat-panel__input"
            type="text"
            maxlength="500"
            prop:value=move || composer.get()
            on:input=move |ev| composer.set(event_target_value(&ev))
            placeholder=move || t_string!(i18n, theater.chat_input_placeholder)
            disabled=is_muted
            aria-label=move || t_string!(i18n, theater.chat_input_placeholder)
            data-testid="theater-chat-input"
          />
          <button
            type="submit"
            class="btn btn--primary"
            disabled=is_muted
            data-testid="theater-chat-send"
          >
            {t!(i18n, theater.chat_send)}
          </button>
        </div>
        <Show when=move || error.get().is_some()>
          <p class="theater-chat-panel__error" role="alert">
            {move || error.get().unwrap_or_default()}
          </p>
        </Show>
      </form>
    </section>
  }
}
