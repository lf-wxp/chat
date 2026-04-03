//! Chat list component

use leptos::prelude::*;
use leptos_i18n::t_string;

use crate::{
  components::{Avatar, AvatarSize, Badge, EmptyState},
  i18n::*,
  state,
};

/// Chat list
#[component]
pub(super) fn ChatList() -> impl IntoView {
  let chat_state = state::use_chat_state();
  // Context menu state: (visible, x, y, conversation_id)
  let ctx_menu = RwSignal::new((false, 0i32, 0i32, String::new()));
  let i18n = use_i18n();

  view! {
      <div class="chat-list">
        {move || {
          let state = chat_state.get();
          let conversations = &state.conversations;
          if conversations.is_empty() {
            view! {
              <EmptyState
                icon="💬"
                title=t_string!(i18n, chat_no_conversations).to_string()
                description=""
              />
            }.into_any()
          } else {
            // Sort: pinned first, then by last message time descending
            let mut sorted: Vec<_> = conversations.clone();
            sorted.sort_by(|a, b| {
              b.pinned.cmp(&a.pinned).then_with(|| b.last_time.cmp(&a.last_time))
            });
            sorted.iter().map(|conv| {
              let name = conv.name.clone();
              let last_msg = conv.last_message.clone().unwrap_or_default();
              let unread = conv.unread_count;
              let conv_id = conv.id.clone();
              let is_pinned = conv.pinned;
              let is_muted = conv.muted;
              let name_for_aria = name.clone();
              let name_for_avatar = name.clone();
              let item_class = format!(
                "chat-list-item{}",
                if is_pinned { " chat-list-item-pinned" } else { "" }
              );
              let conv_id_for_ctx = conv_id.clone();
              view! {
                <div
                  class=item_class
                  tabindex=0
  aria-label=t_string!(i18n, chat_conversation_with).to_string().replace("{}", &name_for_aria)
                  on:contextmenu=move |ev: web_sys::MouseEvent| {
                    ev.prevent_default();
                    ctx_menu.set((true, ev.client_x(), ev.client_y(), conv_id_for_ctx.clone()));
                  }
                >
                  <Avatar username=name_for_avatar size=AvatarSize::Small />
                  <div class="chat-list-item-info">
                    <div class="chat-list-item-name truncate">
                      {if is_pinned { "📌 " } else { "" }}
                      {name}
                    </div>
                    <div class="chat-list-item-preview truncate text-secondary text-sm">{last_msg}</div>
                  </div>
                  <div class="chat-list-item-badges">
                    {if is_muted {
                      view! { <span class="conv-muted-icon" title=t_string!(i18n, chat_mute)>"🔕"</span> }.into_any()
                    } else {
                      ().into_any()
                    }}
                    {if unread > 0 {
                      if is_muted {
                      // Show dot instead of number when muted
                        view! { <span class="conv-muted-dot"></span> }.into_any()
                      } else {
                        view! { <Badge count=unread /> }.into_any()
                      }
                    } else {
                      ().into_any()
                    }}
                  </div>
                </div>
              }
            }).collect_view().into_any()
          }
        }}

        // Context menu: pin/mute operations
        {move || {
          let (show, x, y, conv_id) = ctx_menu.get();
          if !show {
            return view! { <div class="conv-ctx-hidden"></div> }.into_any();
          }
          // Find current conversation's pin/mute status
          let state = chat_state.get();
          let conv = state.conversations.iter().find(|c| c.id == conv_id);
          let is_pinned = conv.is_some_and(|c| c.pinned);
          let is_muted = conv.is_some_and(|c| c.muted);

          let conv_id_pin = conv_id.clone();
          let conv_id_mute = conv_id.clone();

          view! {
            <div class="context-menu-backdrop" on:click=move |_| ctx_menu.set((false, 0, 0, String::new()))>
              <div
                class="context-menu conv-context-menu"
                style=format!("left:{}px;top:{}px", x, y)
                on:click=move |ev: web_sys::MouseEvent| ev.stop_propagation()
              >
                // Pin/unpin
                <button
                  class="context-menu-item"
                  on:click={
                    let conv_id = conv_id_pin.clone();
                    move |_| {
                      chat_state.update(|s| {
                        if let Some(c) = s.conversations.iter_mut().find(|c| c.id == conv_id) {
                          c.pinned = !c.pinned;
                          // Persist
                          crate::storage::persist_conversation(c.clone());
                        }
                      });
                      ctx_menu.set((false, 0, 0, String::new()));
                    }
                  }
                >
                  <span class="context-menu-icon">{if is_pinned { "📍" } else { "📌" }}</span>
                  {if is_pinned { t_string!(i18n, chat_unpin) } else { t_string!(i18n, chat_pin) }}
                </button>
                // Mute/unmute
                <button
                  class="context-menu-item"
                  on:click={
                    let conv_id = conv_id_mute.clone();
                    move |_| {
                      chat_state.update(|s| {
                        if let Some(c) = s.conversations.iter_mut().find(|c| c.id == conv_id) {
                          c.muted = !c.muted;
                          crate::storage::persist_conversation(c.clone());
                        }
                      });
                      ctx_menu.set((false, 0, 0, String::new()));
                    }
                  }
                >
                  <span class="context-menu-icon">{if is_muted { "🔔" } else { "🔕" }}</span>
                  {if is_muted { t_string!(i18n, chat_unmute) } else { t_string!(i18n, chat_mute) }}
                </button>
              </div>
            </div>
          }.into_any()
        }}
      </div>
    }
}
