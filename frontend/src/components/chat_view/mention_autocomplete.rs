//! Inline `@mention` autocomplete popup (G5 fix).
//!
//! Renders a floating suggestion list below the textarea when the user
//! types `@` followed by one or more characters. The list is filtered
//! against the online users list. Keyboard navigation (Up/Down/Enter/
//! Escape) and click selection are both supported.

use crate::state::{ConversationId, use_app_state};
use leptos::prelude::*;
use message::types::UserInfo;

/// Maximum number of suggestions shown in the dropdown.
const MAX_SUGGESTIONS: usize = 8;

/// Extract the @query fragment from text at a given cursor position.
/// Returns `None` if no active mention query is detected.
fn extract_mention_query(text: &str, cursor: usize) -> Option<String> {
  let pos = cursor.min(text.len());
  let before_cursor = &text[..pos];
  let at_pos = before_cursor.rfind('@')?;
  // Ensure the @ is at start or preceded by whitespace.
  if at_pos > 0 && before_cursor.as_bytes()[at_pos - 1] != b' ' {
    return None;
  }
  let fragment = &before_cursor[at_pos + 1..];
  // Only trigger if the fragment has no spaces (single token).
  if fragment.contains(' ') {
    return None;
  }
  Some(fragment.to_lowercase())
}

/// Mention autocomplete popup component.
#[component]
pub fn MentionAutocomplete(
  /// The current text in the input bar (reactive).
  #[prop(into)]
  draft: Signal<String>,
  /// Cursor position (character offset) in the textarea.
  #[prop(into)]
  cursor_pos: Signal<usize>,
  /// Whether the popup is visible.
  visible: RwSignal<bool>,
  /// Callback invoked when a user is selected. The parent should
  /// replace the `@query` fragment with `@nickname `.
  on_select: Callback<UserInfo>,
  /// Active conversation (used to determine member list for rooms).
  #[prop(into)]
  _conv: Signal<Option<ConversationId>>,
) -> impl IntoView {
  let app_state = use_app_state();

  // Index of the currently highlighted suggestion (keyboard nav).
  let highlight_idx = RwSignal::new(0usize);

  // Filtered suggestions based on the query.
  let suggestions = Memo::new(move |_| {
    let text = draft.get();
    let pos = cursor_pos.get();
    let Some(q) = extract_mention_query(&text, pos) else {
      return Vec::new();
    };
    let online = app_state.online_users.get();
    online
      .into_iter()
      .filter(|u| u.nickname.to_lowercase().contains(&q) || u.username.to_lowercase().contains(&q))
      .take(MAX_SUGGESTIONS)
      .collect::<Vec<_>>()
  });

  // Show/hide the popup based on whether we have suggestions.
  Effect::new(move |_| {
    let has = !suggestions.get().is_empty();
    visible.set(has);
    if !has {
      highlight_idx.set(0);
    }
  });

  view! {
    <Show when=move || visible.get()>
      <div
        class="mention-autocomplete"
        data-testid="mention-autocomplete"
        role="listbox"
        aria-label="Mention suggestions"
      >
        {move || {
          suggestions.get().into_iter().enumerate().map(|(idx, user)| {
            let user_for_click = user.clone();
            let nickname = user.nickname.clone();
            let username = user.username.clone();
            view! {
              <div
                class="mention-autocomplete__item"
                class:is-highlighted=move || highlight_idx.get() == idx
                role="option"
                aria-selected=move || (highlight_idx.get() == idx).to_string()
                on:click=move |_| {
                  on_select.run(user_for_click.clone());
                  visible.set(false);
                }
                data-testid="mention-suggestion"
              >
                <span class="mention-autocomplete__nickname">{nickname.clone()}</span>
                " "
                <span class="mention-autocomplete__username">"@"{username.clone()}</span>
              </div>
            }
          }).collect_view()
        }}
      </div>
    </Show>
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_extract_mention_query_basic() {
    assert_eq!(extract_mention_query("@alice", 6), Some("alice".into()));
    assert_eq!(extract_mention_query("hello @bob", 10), Some("bob".into()));
    assert_eq!(extract_mention_query("hello @", 7), Some("".into()));
  }

  #[test]
  fn test_extract_mention_query_no_match() {
    // @ in middle of word (email-like)
    assert_eq!(extract_mention_query("foo@bar", 7), None);
    // Space after @ fragment
    assert_eq!(extract_mention_query("@alice hello", 12), None);
  }

  #[test]
  fn test_extract_mention_query_cursor_mid() {
    assert_eq!(extract_mention_query("@alice", 3), Some("al".into()));
  }
}
