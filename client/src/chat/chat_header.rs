//! Chat header component

use leptos::prelude::*;
use leptos_i18n::{t, t_string};

use crate::{
  components::{Avatar, AvatarSize},
  i18n::*,
  state,
};

/// Chat header with peer info and action buttons
#[component]
pub fn ChatHeader(
  /// Peer username
  #[prop(into)]
  peer_name: String,
  /// Signal to toggle in-chat search visibility
  show_chat_search: RwSignal<bool>,
  /// Signal to clear search query
  chat_search_query: RwSignal<String>,
) -> impl IntoView {
  let i18n = use_i18n();
  let search_state = state::use_search_state();

  view! {
    <div class="chat-header">
      <Avatar username=peer_name.clone() size=AvatarSize::Small online=true />
      <div class="flex-1">
        <div class="font-medium">{peer_name}</div>
        <div class="text-xs text-secondary">{t!(i18n, common_online)}</div>
      </div>
      <div class="chat-header-actions">
        <button
          class="tool-btn"
          tabindex=0
          aria-label=move || t_string!(i18n, chat_search_messages)
          on:click=move |_| {
            let showing = show_chat_search.get_untracked();
            show_chat_search.set(!showing);
            if showing {
              // Clear state when closing search
              chat_search_query.set(String::new());
              search_state.update(|s| {
                s.in_chat_matches.clear();
                s.in_chat_current_index = 0;
                s.query.clear();
              });
            }
          }
        >"🔍"</button>
        <button class="tool-btn" tabindex=0 aria-label=move || t_string!(i18n, chat_voice_call)>"📞"</button>
        <button class="tool-btn" tabindex=0 aria-label=move || t_string!(i18n, chat_video_call)>"📹"</button>
      </div>
    </div>
  }
}
