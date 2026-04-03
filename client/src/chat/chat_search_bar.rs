//! In-conversation search bar component

use leptos::prelude::*;
use leptos_i18n::{t, t_string};

use crate::{i18n::*, state};

/// In-conversation search bar
#[component]
pub fn ChatSearchBar(
  /// Whether to show the search bar
  show_chat_search: RwSignal<bool>,
  /// Search query text
  chat_search_query: RwSignal<String>,
) -> impl IntoView {
  let i18n = use_i18n();
  let search_state = state::use_search_state();
  let chat_state = state::use_chat_state();

  move || {
    if !show_chat_search.get() {
      return view! { <div class="chat-search-hidden"></div> }.into_any();
    }
    let match_count = search_state.get().in_chat_matches.len();
    let current_idx = search_state.get().in_chat_current_index;
    view! {
      <div class="chat-search-bar">
        <div class="chat-search-input-wrap">
          <span class="chat-search-icon">"🔍"</span>
          <input
            class="chat-search-input"
            type="text"
            placeholder=move || t_string!(i18n, chat_search_conversation)
            prop:value=move || chat_search_query.get()
            on:input=move |ev| {
              let val = event_target_value(&ev);
              chat_search_query.set(val.clone());
              // Real-time search in current conversation messages
              let query_lower = val.to_lowercase();
              if query_lower.is_empty() {
                search_state.update(|s| {
                  s.in_chat_matches.clear();
                  s.in_chat_current_index = 0;
                  s.query.clear();
                });
              } else {
                let messages = chat_state.get_untracked().messages.clone();
                let matches: Vec<String> = messages.iter().filter(|m| {
                  match &m.content {
                    message::chat::MessageContent::Text(t) | message::chat::MessageContent::System(t) => t.to_lowercase().contains(&query_lower),
                    _ => false,
                  }
                }).map(|m| m.id.clone()).collect();
                let idx = if matches.is_empty() { 0 } else { matches.len() - 1 };
                search_state.update(|s| {
                  s.in_chat_matches = matches;
                  s.in_chat_current_index = idx;
                  s.query = val;
                });
              }
            }
            on:keydown=move |ev: web_sys::KeyboardEvent| {
              if ev.key() == "Enter" {
                ev.prevent_default();
                let count = search_state.get_untracked().in_chat_matches.len();
                if count > 0 {
                  if ev.shift_key() {
                    // Shift+Enter → previous match
                    search_state.update(|s| {
                      if s.in_chat_current_index > 0 {
                        s.in_chat_current_index -= 1;
                      } else {
                        s.in_chat_current_index = count - 1;
                      }
                    });
                  } else {
                    // Enter → next match
                    search_state.update(|s| {
                      s.in_chat_current_index = (s.in_chat_current_index + 1) % count;
                    });
                  }
                }
              } else if ev.key() == "Escape" {
                show_chat_search.set(false);
                chat_search_query.set(String::new());
                search_state.update(|s| {
                  s.in_chat_matches.clear();
                  s.in_chat_current_index = 0;
                  s.query.clear();
                });
              }
            }
          />
        </div>
        <div class="chat-search-nav">
          {if match_count > 0 {
            view! {
              <span class="chat-search-count">{format!("{}/{}", current_idx + 1, match_count)}</span>
            }.into_any()
          } else if !chat_search_query.get().is_empty() {
            view! {
              <span class="chat-search-count no-match">{t!(i18n, chat_no_match)}</span>
            }.into_any()
          } else {
            view! { <span></span> }.into_any()
          }}
          <button
            class="chat-search-nav-btn"
            tabindex=0
            aria-label=move || t_string!(i18n, chat_prev_match)
            on:click=move |_| {
              let count = search_state.get_untracked().in_chat_matches.len();
              if count > 0 {
                search_state.update(|s| {
                  if s.in_chat_current_index > 0 {
                    s.in_chat_current_index -= 1;
                  } else {
                    s.in_chat_current_index = count - 1;
                  }
                });
              }
            }
          >"▲"</button>
          <button
            class="chat-search-nav-btn"
            tabindex=0
            aria-label=move || t_string!(i18n, chat_next_match)
            on:click=move |_| {
              let count = search_state.get_untracked().in_chat_matches.len();
              if count > 0 {
                search_state.update(|s| {
                  s.in_chat_current_index = (s.in_chat_current_index + 1) % count;
                });
              }
            }
          >"▼"</button>
          <button
            class="chat-search-close-btn"
            tabindex=0
            aria-label=move || t_string!(i18n, chat_close_search)
            on:click=move |_| {
              show_chat_search.set(false);
              chat_search_query.set(String::new());
              search_state.update(|s| {
                s.in_chat_matches.clear();
                s.in_chat_current_index = 0;
                s.query.clear();
              });
            }
          >"✕"</button>
        </div>
      </div>
    }.into_any()
  }
}
