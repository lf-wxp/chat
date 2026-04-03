//! Message list component

use leptos::prelude::*;
use leptos_i18n::t_string;
use wasm_bindgen::JsCast;

use message::chat::ChatMessage;

use crate::{
  components::EmptyState,
  i18n::*,
  state,
};

use super::message_bubble::{ContextAction, MessageBubble};

/// Message list display
#[component]
pub fn MessageList(
  /// Signal for the message being replied to
  reply_to_msg: RwSignal<Option<ChatMessage>>,
) -> impl IntoView {
  let i18n = use_i18n();
  let chat_state = state::use_chat_state();
  let user_state = state::use_user_state();
  let search_state = state::use_search_state();

  view! {
    <div class="chat-messages" id="chat-messages-container">
      {move || {
        let messages = &chat_state.get().messages;
        let my_id = user_state.get_untracked().user_id.clone();
        let search = search_state.get();
        let current_highlight_id = search.in_chat_matches
          .get(search.in_chat_current_index)
          .cloned();

        if messages.is_empty() {
          view! {
            <EmptyState
              icon="💬"
              title=t_string!(i18n, chat_start).to_string()
              description=t_string!(i18n, chat_send_first_message).to_string()
            />
          }.into_any()
        } else {
          messages.iter().map(|msg| {
            let is_mine = msg.from == my_id;
            let msg_clone = msg.clone();
            let msg_id = msg.id.clone();
            let is_match = search.in_chat_matches.contains(&msg_id);
            let is_current = current_highlight_id.as_deref() == Some(&*msg_id);
            let highlight_class = if is_current {
              "search-highlight-current"
            } else if is_match {
              "search-highlight"
            } else {
              ""
            };
            // Auto-scroll to current highlighted message
            if is_current {
              let id = msg_id.clone();
              // Delay scroll to ensure DOM is updated
              let cb = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
                if let Some(window) = web_sys::window()
                  && let Some(document) = window.document()
                  && let Some(el) = document.get_element_by_id(&format!("msg-{id}"))
                {
                  // Use JS to call scrollIntoView({ behavior: 'smooth', block: 'center' })
                  let _ = js_sys::Reflect::apply(
                    &js_sys::Function::new_no_args(""),
                    &el,
                    &js_sys::Array::new(),
                  );
                  // Simple scroll into view
                  el.scroll_into_view_with_bool(true);
                }
              });
              let _ = web_sys::window().unwrap().set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(), 50,
              );
              cb.forget();
            }
            let on_action = Callback::new(move |action: ContextAction| {
              match action {
                ContextAction::Reply(m) => {
                  reply_to_msg.set(Some(m));
                }
                ContextAction::Copy(_) => {
                  // Copy operation handled inside MessageBubble
                }
                ContextAction::Recall(msg_id) => {
                  // Remove recalled message from message list
                  chat_state.update(|s| {
                    s.messages.retain(|m| m.id != msg_id);
                  });
                }
              }
            });
            view! {
              <div id=format!("msg-{}", msg_id) class=highlight_class>
                <MessageBubble message=msg_clone is_mine=is_mine on_action=on_action />
              </div>
            }
          }).collect_view().into_any()
        }
      }}
    </div>
  }
}
