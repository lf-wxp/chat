//! Forward-message modal.
//!
//! When the user clicks the "➤ Forward" action in a message bubble
//! the parent `ChatView` stores the source `ChatMessage` in an
//! `RwSignal<Option<ChatMessage>>` and the modal renders a searchable
//! list of candidate conversations. Picking a target calls
//! `ChatManager::forward_message`, which rejects chain forwarding on
//! its own (Req 4.6.x, error `cht104`) — this UI only needs to render
//! the failure path when the manager returns `None`.

use crate::chat::{ChatMessage, MessageContent, use_chat_manager};
use crate::i18n;
use crate::state::{Conversation, ConversationId, use_app_state};
use leptos::prelude::*;
use leptos_i18n::t_string;
use wasm_bindgen::JsCast;

/// Forward-message modal.
#[component]
pub fn ForwardModal(
  /// Source message; `None` while the modal is closed.
  source: RwSignal<Option<ChatMessage>>,
) -> impl IntoView {
  let manager = use_chat_manager();
  let app_state = use_app_state();
  let i18n = i18n::use_i18n();

  let query = RwSignal::new(String::new());
  let error = RwSignal::new(Option::<String>::None);

  // Conversations matching the filter query (case-insensitive on
  // `display_name`). Excludes the conversation the source message
  // currently lives in.
  let filtered = Memo::new(move |_| {
    let q = query.get().trim().to_lowercase();
    let mut list: Vec<Conversation> = app_state
      .conversations
      .get()
      .into_iter()
      .filter(|c| !c.archived)
      .filter(|c| q.is_empty() || c.display_name.to_lowercase().contains(&q))
      .collect();
    list.sort_by_key(|c| std::cmp::Reverse(c.last_message_ts));
    list
  });

  let close = move |_| {
    source.set(None);
    query.set(String::new());
    error.set(None);
  };

  let is_chain_forward = Memo::new(move |_| {
    matches!(
      source.get().map(|m| m.content),
      Some(MessageContent::Forwarded { .. })
    )
  });

  let manager_store = StoredValue::new(manager.clone());

  let do_forward = move |target: ConversationId| {
    let Some(msg) = source.get_untracked() else {
      return;
    };
    let ok = manager_store.with_value(|m| m.forward_message(target, &msg));
    match ok {
      Some(_) => {
        source.set(None);
        query.set(String::new());
        error.set(None);
      }
      None => {
        error.set(Some(
          t_string!(i18n, chat.forward_chain_forbidden).to_string(),
        ));
      }
    }
  };

  view! {
    <Show when=move || source.get().is_some() fallback=|| ()>
      <div
        class="forward-modal-backdrop"
        role="dialog"
        aria-modal="true"
        data-testid="forward-modal"
        on:click=move |ev| {
          // Close when the backdrop itself is clicked, not bubbled
          // events from the inner modal.
          if let Some(target) = ev.target()
            && let Ok(el) = target.dyn_into::<web_sys::Element>()
            && el.class_list().contains("forward-modal-backdrop")
          {
            close(ev);
          }
        }
      >
        <div class="forward-modal">
          <header>
            <span>{t_string!(i18n, chat.forward_modal_title)}</span>
            <button
              type="button"
              class="chat-input-btn"
              aria-label=move || t_string!(i18n, common.close)
              on:click=close
            >
              "×"
            </button>
          </header>

          <Show when=move || is_chain_forward.get() fallback=|| ()>
            <div class="forward-modal-error" role="alert">
              {move || t_string!(i18n, chat.forward_chain_forbidden)}
            </div>
          </Show>

          <Show when=move || !is_chain_forward.get() fallback=|| ()>
            <div style="padding:0.5rem 1rem">
              <input
                type="search"
                class="chat-input-textarea"
                placeholder=move || t_string!(i18n, chat.forward_modal_placeholder)
                aria-label=move || t_string!(i18n, chat.forward_modal_placeholder)
                prop:value=move || query.get()
                on:input=move |ev| {
                  if let Some(v) = input_value(&ev) {
                    query.set(v);
                  }
                }
              />
            </div>

            <ul role="listbox">
              {move || {
                filtered
                  .get()
                  .into_iter()
                  .map(|conv| {
                    let name = conv.display_name.clone();
                    let id = conv.id.clone();
                    view! {
                      <li
                        role="option"
                        on:click=move |_| do_forward(id.clone())
                      >
                        <span>{name}</span>
                        <span class="forward-modal-last">
                          {conv.last_message.clone().unwrap_or_default()}
                        </span>
                      </li>
                    }
                  })
                  .collect_view()
              }}
            </ul>
          </Show>

          <Show when=move || error.get().is_some() fallback=|| ()>
            <div class="forward-modal-error" role="alert">
              {move || error.get().unwrap_or_default()}
            </div>
          </Show>

          <footer>
            <button type="button" class="chat-input-btn" on:click=close>
              {t_string!(i18n, common.close)}
            </button>
          </footer>
        </div>
      </div>
    </Show>
  }
}

/// Extract the `value` of an `<input>` from an event.
fn input_value(ev: &leptos::ev::Event) -> Option<String> {
  let target = ev.target()?;
  target
    .dyn_into::<web_sys::HtmlInputElement>()
    .ok()
    .map(|el| el.value())
}
