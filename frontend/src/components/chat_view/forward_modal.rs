//! Forward-message modal.
//!
//! When the user clicks the "➤ Forward" action in a message bubble
//! the parent `ChatView` stores the source `ChatMessage` in an
//! `RwSignal<Option<ChatMessage>>` and the modal renders a searchable
//! list of candidate conversations. Picking a target calls
//! `ChatManager::forward_message`, which rejects chain forwarding on
//! its own (Req 4.6.x, error `cht104`) — this UI only needs to render
//! the failure path when the manager returns `None`.
//!
//! Layout / animation / dismissal are delegated to the shared
//! `ModalWrapper` so the visual + interaction model matches every
//! other dialog in the app.

use crate::chat::{ChatMessage, MessageContent, use_chat_manager};
use crate::components::room::modal_wrapper::{ModalSize, ModalWrapper};
use crate::i18n;
use crate::state::{Conversation, ConversationId, use_app_state};
use icondata as i;
use leptos::prelude::*;
use leptos_i18n::t_string;
use leptos_icons::Icon;
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

  // Derived signal that drives ModalWrapper's `open` prop.
  let is_open = Signal::derive(move || source.with(Option::is_some));

  // Conversations matching the filter query (case-insensitive on
  // `display_name`). Excludes archived conversations.
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

  let reset = move || {
    query.set(String::new());
    error.set(None);
  };

  let on_close = Callback::new(move |()| {
    source.set(None);
    reset();
  });

  let close_btn = move |_| {
    source.set(None);
    reset();
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
        reset();
      }
      None => {
        error.set(Some(
          t_string!(i18n, chat.forward_chain_forbidden).to_string(),
        ));
      }
    }
  };

  view! {
    <ModalWrapper
      on_close=on_close
      open=is_open
      size=ModalSize::Small
      class="forward-modal"
      labelled_by="forward-modal-title"
      testid="forward-modal"
    >
      <header class="modal-header">
        <h2 id="forward-modal-title" class="modal-title">
          {move || t_string!(i18n, chat.forward_modal_title)}
        </h2>
        <button
          type="button"
          class="modal-close"
          aria-label=move || t_string!(i18n, common.close)
          on:click=close_btn
        >
          <Icon icon=i::LuX />
        </button>
      </header>

      <Show when=move || is_chain_forward.get() fallback=|| ()>
        <div class="forward-modal-error" role="alert">
          {move || t_string!(i18n, chat.forward_chain_forbidden)}
        </div>
      </Show>

      <Show when=move || !is_chain_forward.get() fallback=|| ()>
        <div class="modal-body">
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
        </div>
      </Show>

      <Show when=move || error.get().is_some() fallback=|| ()>
        <div class="forward-modal-error" role="alert">
          {move || error.get().unwrap_or_default()}
        </div>
      </Show>

      <footer class="modal-footer">
        <button type="button" class="btn btn--ghost" on:click=close_btn>
          {move || t_string!(i18n, common.close)}
        </button>
      </footer>
    </ModalWrapper>
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
