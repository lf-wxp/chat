//! Chat search functionality
//!
//! Provides in-conversation message search with highlighting and navigation.

use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::state;

/// Chat search state
pub struct ChatSearchState {
  /// Whether search bar is visible
  pub show_search: RwSignal<bool>,
  /// Search query text
  pub query: RwSignal<String>,
}

impl ChatSearchState {
  /// Create new chat search state
  pub fn new() -> Self {
    Self {
      show_search: RwSignal::new(false),
      query: RwSignal::new(String::new()),
    }
  }

  /// Toggle search visibility
  pub fn toggle(&self) {
    let showing = self.show_search.get_untracked();
    self.show_search.set(!showing);
    if showing {
      // Clear state when closing
      self.query.set(String::new());
      let search_state = state::use_search_state();
      search_state.update(|s| {
        s.in_chat_matches.clear();
        s.in_chat_current_index = 0;
        s.query.clear();
      });
    }
  }

  /// Close search and clear state
  pub fn close(&self) {
    self.show_search.set(false);
    self.query.set(String::new());
    let search_state = state::use_search_state();
    search_state.update(|s| {
      s.in_chat_matches.clear();
      s.in_chat_current_index = 0;
      s.query.clear();
    });
  }
}

impl Default for ChatSearchState {
  fn default() -> Self {
    Self::new()
  }
}

/// Perform search on chat messages
pub fn perform_search(query: &str) {
  let search_state = state::use_search_state();
  let chat_state = state::use_chat_state();

  let query_lower = query.to_lowercase();

  if query_lower.is_empty() {
    search_state.update(|s| {
      s.in_chat_matches.clear();
      s.in_chat_current_index = 0;
      s.query.clear();
    });
  } else {
    let messages = chat_state.get_untracked().messages.clone();
    let matches: Vec<String> = messages
      .iter()
      .filter(|m| match &m.content {
        message::chat::MessageContent::Text(t) | message::chat::MessageContent::System(t) => {
          t.to_lowercase().contains(&query_lower)
        }
        _ => false,
      })
      .map(|m| m.id.clone())
      .collect();
    let idx = if matches.is_empty() {
      0
    } else {
      matches.len() - 1
    };
    search_state.update(|s| {
      s.in_chat_matches = matches;
      s.in_chat_current_index = idx;
      s.query = query.to_string();
    });
  }
}

/// Navigate to previous match
pub fn navigate_prev() {
  let search_state = state::use_search_state();
  search_state.update(|s| {
    let count = s.in_chat_matches.len();
    if count > 0 {
      if s.in_chat_current_index > 0 {
        s.in_chat_current_index -= 1;
      } else {
        s.in_chat_current_index = count - 1;
      }
    }
  });
}

/// Navigate to next match
pub fn navigate_next() {
  let search_state = state::use_search_state();
  search_state.update(|s| {
    let count = s.in_chat_matches.len();
    if count > 0 {
      s.in_chat_current_index = (s.in_chat_current_index + 1) % count;
    }
  });
}

/// Get current highlight message ID
pub fn get_current_highlight_id() -> Option<String> {
  let search_state = state::use_search_state();
  let search = search_state.get();
  search
    .in_chat_matches
    .get(search.in_chat_current_index)
    .cloned()
}

/// Scroll to message element by ID
pub fn scroll_to_message(msg_id: &str) {
  let id = msg_id.to_string();
  let cb = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
    if let Some(window) = web_sys::window()
      && let Some(document) = window.document()
      && let Some(el) = document.get_element_by_id(&format!("msg-{id}"))
    {
      el.scroll_into_view_with_bool(true);
    }
  });
  let _ = web_sys::window()
    .unwrap()
    .set_timeout_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), 50);
  cb.forget();
}

/// Handle search input keydown events
pub fn handle_search_keydown(ev: &web_sys::KeyboardEvent, search_state: &ChatSearchState) {
  if ev.key() == "Enter" {
    ev.prevent_default();
    let count = state::use_search_state()
      .get_untracked()
      .in_chat_matches
      .len();
    if count > 0 {
      if ev.shift_key() {
        // Shift+Enter -> previous
        navigate_prev();
      } else {
        // Enter -> next
        navigate_next();
      }
    }
  } else if ev.key() == "Escape" {
    search_state.close();
  }
}
