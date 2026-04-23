//! Tiny emoji picker used by the reaction button on each message.
//!
//! The picker is intentionally small and self-contained: a 16-glyph
//! grid that toggles a reaction on the target message when clicked.
//! Messages cap at 20 distinct emoji, so a short curated list is fine
//! here; users that need more can forward a proper sticker instead.

use crate::chat::use_chat_manager;
use crate::i18n;
use leptos::prelude::*;
use leptos_i18n::t_string;
use message::MessageId;

/// Short curated reaction set. Matches the most common reactions used
/// by other mainstream chat clients.
const CURATED_EMOJI: &[&str] = &[
  "👍", "❤️", "😂", "😮", "😢", "🎉", "🙏", "🔥", "👀", "👏", "💯", "🤔", "✅", "❌", "🚀", "🥳",
];

/// Emoji reaction picker.
#[component]
pub fn ReactionPicker(
  /// Target message id.
  message_id: MessageId,
  /// Invoked after a successful toggle so the caller can close the
  /// picker.
  #[prop(into)]
  on_close: Callback<()>,
) -> impl IntoView {
  let manager = use_chat_manager();
  let i18n = i18n::use_i18n();

  view! {
    <div class="emoji-picker" data-testid="reaction-picker" role="menu">
      {CURATED_EMOJI
        .iter()
        .map(|emoji| {
          let emoji = (*emoji).to_string();
          let label = t_string!(i18n, chat.add_reaction).to_string();
          let emoji_for_click = emoji.clone();
          let emoji_for_display = emoji.clone();
          view! {
            <button
              type="button"
              aria-label=label
              on:click={
                let emoji = emoji_for_click.clone();
                let manager = manager.clone();
                move |_| {
                  let _ = manager.toggle_reaction(message_id, emoji.clone());
                  on_close.run(());
                }
              }
            >
              {emoji_for_display}
            </button>
          }
        })
        .collect_view()}
    </div>
  }
}
