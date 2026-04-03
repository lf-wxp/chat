//! Reply preview bar component

use leptos::prelude::*;
use leptos_i18n::t_string;

use message::chat::{ChatMessage, MessageContent};

use crate::i18n::*;

/// Reply preview bar shown above the input area
#[component]
pub fn ReplyBar(
  /// The message being replied to
  reply_to_msg: RwSignal<Option<ChatMessage>>,
) -> impl IntoView {
  let i18n = use_i18n();

  move || {
    reply_to_msg.get().map(|msg| {
      let preview = match &msg.content {
        MessageContent::Text(t) => {
          let truncated: String = t.chars().take(50).collect();
          if t.chars().count() > 50 { format!("{truncated}...") } else { truncated }
        }
        MessageContent::Image { .. } => t_string!(i18n, chat_image).to_string(),
        MessageContent::Voice { .. } => t_string!(i18n, chat_voice).to_string(),
        MessageContent::File(f) => format!("{} {}", t_string!(i18n, chat_file), f.name),
        MessageContent::Sticker { .. } => t_string!(i18n, chat_sticker).to_string(),
        MessageContent::System(t) => t.clone(),
      };
      let reply_text = format!("{} {}: {}", t_string!(i18n, chat_reply_prefix), msg.from, preview);
      view! {
        <div class="reply-bar">
          <div class="reply-bar-content">
            <span class="reply-bar-icon">"↩️"</span>
            <span class="reply-bar-text">{reply_text}</span>
          </div>
          <button
            class="reply-bar-close"
            tabindex=0
            aria-label=move || t_string!(i18n, chat_cancel_reply)
            on:click=move |_| reply_to_msg.set(None)
          >
            "✕"
          </button>
        </div>
      }
    })
  }
}
