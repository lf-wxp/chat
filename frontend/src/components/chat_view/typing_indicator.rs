//! Typing-indicator strip rendered below the message list.
//!
//! Shows a lightweight "{name} is typing..." / "Several people are
//! typing..." hint sourced from the chat manager's per-conversation
//! `typing` signal.

use crate::chat::use_chat_manager;
use crate::i18n;
use crate::state::ConversationId;
use leptos::prelude::*;
use leptos_i18n::t_string;

/// Typing indicator strip.
#[component]
pub fn TypingIndicator(conv: Signal<Option<ConversationId>>) -> impl IntoView {
  let manager = use_chat_manager();
  let i18n = i18n::use_i18n();

  // Derived signal that returns the rendered label or an empty string
  // when nobody is typing.
  let label = {
    let manager = manager.clone();
    Signal::derive(move || {
      let Some(id) = conv.get() else {
        return String::new();
      };
      let state = manager.conversation_state(&id);
      let names = state.typing.get();
      match names.len() {
        0 => String::new(),
        1 => format!("{}{}", names[0], t_string!(i18n, chat.typing_indicator)),
        _ => t_string!(i18n, chat.typing_multiple).to_string(),
      }
    })
  };

  // Reactive signal: `true` while at least one peer in the active
  // conversation is typing. Drives the `hidden` attribute on the
  // outer `<div>` so the indicator collapses to zero height (and
  // becomes "hidden" from Playwright's `toBeHidden` perspective)
  // when nobody is typing.
  let is_typing = Signal::derive(move || {
    let Some(id) = conv.get() else {
      return false;
    };
    let state = manager.conversation_state(&id);
    !state.typing.get().is_empty()
  });

  view! {
    <div
      class="typing-indicator"
      data-testid="typing-indicator"
      hidden=move || !is_typing.get()
    >
      // Animated three-dot pulse. Purely decorative; the label carries
      // the accessible text. The outer `.typing-dots` provides the
      // ::before/::after dots; the inner <span> is the middle dot.
      <span class="typing-dots" aria-hidden="true">
        <span></span>
      </span>
      {move || label.get()}
    </div>
  }
}
