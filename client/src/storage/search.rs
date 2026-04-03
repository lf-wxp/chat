//! Message search functionality

use js_sys::{Array, Reflect};
use leptos::prelude::GetUntracked;
use message::chat::ChatMessage;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{IdbDatabase, IdbTransactionMode};

use crate::state::SearchResultItem;

use super::db::{MESSAGES_STORE, open_db, wait_request};

/// Search messages across all conversations (keyword matching on text content)
///
/// Iterates over all messages and returns those whose text content contains the keyword.
pub async fn search_messages_all(
  db: &IdbDatabase,
  query: &str,
) -> Result<Vec<(ChatMessage, String)>, JsValue> {
  let tx = db.transaction_with_str_and_mode(MESSAGES_STORE, IdbTransactionMode::Readonly)?;
  let store = tx.object_store(MESSAGES_STORE)?;
  let request = store.get_all()?;
  let result = wait_request(&request).await?;

  let array: Array = result.unchecked_into();
  let query_lower = query.to_lowercase();
  let mut matches = Vec::new();

  for i in 0..array.length() {
    let js_val = array.get(i);
    // Extract conv_id
    let conv_id = Reflect::get(&js_val, &JsValue::from_str("conv_id"))
      .ok()
      .and_then(|v| v.as_string())
      .unwrap_or_default();

    let json_str = js_sys::JSON::stringify(&js_val)?;
    let json: String = json_str.as_string().unwrap_or_default();
    if let Ok(msg) = serde_json::from_str::<ChatMessage>(&json) {
      // Only search text messages and system messages
      let text = match &msg.content {
        message::chat::MessageContent::Text(t) | message::chat::MessageContent::System(t) => {
          t.clone()
        }
        _ => continue,
      };
      if text.to_lowercase().contains(&query_lower) {
        matches.push((msg, conv_id));
      }
    }
  }

  // Sort by timestamp descending (newest first)
  matches.sort_by(|a, b| b.0.timestamp.cmp(&a.0.timestamp));
  Ok(matches)
}

/// Async search messages and update SearchState (fire-and-forget)
pub fn search_messages_async(
  search_state: leptos::prelude::RwSignal<crate::state::SearchState>,
  chat_state: leptos::prelude::RwSignal<crate::state::ChatState>,
  query: String,
) {
  use leptos::prelude::Update;

  if query.trim().is_empty() {
    search_state.update(|s| {
      s.results.clear();
      s.is_searching = false;
    });
    return;
  }

  search_state.update(|s| s.is_searching = true);

  let query_clone = query.clone();
  wasm_bindgen_futures::spawn_local(async move {
    let db = match open_db().await {
      Ok(db) => db,
      Err(e) => {
        web_sys::console::warn_1(&format!("Search failed — opening IndexedDB: {e:?}").into());
        search_state.update(|s| s.is_searching = false);
        return;
      }
    };

    match search_messages_all(&db, &query_clone).await {
      Ok(matches) => {
        let conversations = chat_state.get_untracked().conversations.clone();
        let results: Vec<SearchResultItem> = matches
          .into_iter()
          .take(50) // Return at most 50 results
          .map(|(msg, conv_id)| {
            let conv_name = conversations
              .iter()
              .find(|c| c.id == conv_id)
              .map_or_else(|| conv_id.clone(), |c| c.name.clone());

            let preview = match &msg.content {
              message::chat::MessageContent::Text(t) | message::chat::MessageContent::System(t) => {
                make_search_preview(t, &query_clone, 40)
              }
              _ => String::new(),
            };

            SearchResultItem {
              message_id: msg.id.clone(),
              conversation_id: conv_id,
              conversation_name: conv_name,
              from: msg.from.clone(),
              preview,
              timestamp: msg.timestamp,
            }
          })
          .collect();

        search_state.update(|s| {
          s.results = results;
          s.is_searching = false;
        });
      }
      Err(e) => {
        web_sys::console::warn_1(&format!("Search failed: {e:?}").into());
        search_state.update(|s| s.is_searching = false);
      }
    }
  });
}

/// Generate search preview text: extract context around the keyword
fn make_search_preview(text: &str, query: &str, context_chars: usize) -> String {
  let lower = text.to_lowercase();
  let query_lower = query.to_lowercase();
  if let Some(pos) = lower.find(&query_lower) {
    let start = pos.saturating_sub(context_chars);
    let end = (pos + query.len() + context_chars).min(text.len());
    // Ensure slicing at character boundaries
    let start = text.floor_char_boundary(start);
    let end = text.ceil_char_boundary(end);
    let mut preview = String::new();
    if start > 0 {
      preview.push_str("...");
    }
    preview.push_str(&text[start..end]);
    if end < text.len() {
      preview.push_str("...");
    }
    preview
  } else {
    text.chars().take(80).collect()
  }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
  use super::*;
  use wasm_bindgen_test::wasm_bindgen_test;

  #[wasm_bindgen_test]
  fn test_make_search_preview_keyword_at_start() {
    let result = make_search_preview("hello world foo bar", "hello", 10);
    assert!(result.contains("hello"));
    // Keyword at the beginning, should not have prefix ellipsis
    assert!(!result.starts_with("..."));
  }

  #[wasm_bindgen_test]
  fn test_make_search_preview_keyword_in_middle() {
    let text = "a".repeat(50) + "KEYWORD" + &"b".repeat(50);
    let result = make_search_preview(&text, "KEYWORD", 10);
    assert!(result.contains("KEYWORD"));
    // Both sides have content truncated
    assert!(result.starts_with("..."));
    assert!(result.ends_with("..."));
  }

  #[wasm_bindgen_test]
  fn test_make_search_preview_keyword_at_end() {
    let text = "a".repeat(50) + "KEYWORD";
    let result = make_search_preview(&text, "KEYWORD", 10);
    assert!(result.contains("KEYWORD"));
    // Keyword at the end, should not have suffix ellipsis
    assert!(!result.ends_with("..."));
  }

  #[wasm_bindgen_test]
  fn test_make_search_preview_case_insensitive() {
    let result = make_search_preview("Hello World", "hello", 40);
    assert!(result.contains("Hello"));
  }

  #[wasm_bindgen_test]
  fn test_make_search_preview_not_found() {
    let text = "a".repeat(100);
    let result = make_search_preview(&text, "xyz", 10);
    // Return first 80 characters when not found
    assert_eq!(result.len(), 80);
  }

  #[wasm_bindgen_test]
  fn test_make_search_preview_short_text() {
    let result = make_search_preview("hi", "hi", 40);
    assert_eq!(result, "hi");
  }

  #[wasm_bindgen_test]
  fn test_make_search_preview() {
    let text =
      "This is a Chinese text containing keywords for testing search preview functionality";
    let result = make_search_preview(text, "keywords", 5);
    assert!(result.contains("keywords"));
  }

  #[wasm_bindgen_test]
  fn test_make_search_preview_empty_query() {
    let result = make_search_preview("hello world", "", 10);
    // Empty query matches position 0
    assert!(result.contains("hello"));
  }

  #[wasm_bindgen_test]
  fn test_make_search_preview_context_zero() {
    let text = "prefix KEYWORD suffix";
    let result = make_search_preview(text, "KEYWORD", 0);
    assert!(result.contains("KEYWORD"));
  }
}
