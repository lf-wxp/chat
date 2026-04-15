//! Sidebar conversation item component.

use crate::state::use_app_state;
use leptos::prelude::*;

/// Sidebar conversation item component.
#[component]
pub fn SidebarConversationItem(conversation: crate::state::Conversation) -> impl IntoView {
  let app_state = use_app_state();
  let conv_id = conversation.id.clone();
  let display_name = conversation.display_name.clone();
  let last_message = conversation.last_message.clone();
  let pinned = conversation.pinned;
  let muted = conversation.muted;
  let unread_count = conversation.unread_count;
  let first_char = display_name.chars().next().unwrap_or('?');

  let conv_id_active = conv_id.clone();
  let is_active =
    Signal::derive(move || app_state.active_conversation.get() == Some(conv_id_active.clone()));

  let item_class = move || {
    if is_active.get() {
      "sidebar-conversation sidebar-item-active"
    } else {
      "sidebar-conversation sidebar-item"
    }
  };

  view! {
    <div
      class=item_class
      tabindex="0"
      role="button"
      aria-label=display_name.clone()
      on:click={
        let conv_id_click = conv_id.clone();
        move |_| {
          app_state.active_conversation.set(Some(conv_id_click.clone()));
        }
      }
      on:keydown={
        let conv_id_key = conv_id.clone();
        move |ev: web_sys::KeyboardEvent| {
          if ev.key() == "Enter" || ev.key() == " " {
            app_state.active_conversation.set(Some(conv_id_key.clone()));
          }
        }
      }
    >
      // Avatar
      <div class="sidebar-conversation-avatar">
        <div class="avatar avatar-sm">
          <span class="text-sm font-semibold">
            {first_char.to_string()}
          </span>
        </div>
      </div>

      // Conversation info
      <div class="sidebar-conversation-info">
        <div class="sidebar-conversation-name truncate">
          {if pinned { "* " } else { "" }}
          {display_name.clone()}
        </div>
        <div class="sidebar-conversation-preview truncate">
          {last_message.unwrap_or_default()}
        </div>
      </div>

      // Unread badge
      {if unread_count > 0 {
        view! {
          <span class="sidebar-item-badge-unread">
            {unread_count}
          </span>
        }.into_any()
      } else {
        view! { <span></span> }.into_any()
      }}

      // Mute indicator
      {if muted {
        view! { <span class="text-tertiary text-xs">"[muted]"</span> }.into_any()
      } else {
        view! { <span></span> }.into_any()
      }}
    </div>
  }
}
