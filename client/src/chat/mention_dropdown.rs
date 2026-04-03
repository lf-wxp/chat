//! @mention dropdown component

use leptos::prelude::*;
use leptos_i18n::t_string;

use crate::{
  components::{Avatar, AvatarSize},
  i18n::*,
  state,
};

use super::mention::insert_mention;

/// @mention user dropdown list
#[component]
pub fn MentionDropdown(
  /// Whether to show the mention list
  show_mention_list: RwSignal<bool>,
  /// Current mention query text
  mention_query: RwSignal<String>,
  /// Selected index in the dropdown
  mention_selected_index: RwSignal<usize>,
  /// Input text signal (for inserting mention)
  input_text: RwSignal<String>,
) -> impl IntoView {
  let i18n = use_i18n();
  let online_users_state = state::use_online_users_state();
  let user_state = state::use_user_state();

  move || {
    if !show_mention_list.get() {
      return view! { <div class="mention-list-hidden"></div> }.into_any();
    }
    let query = mention_query.get().to_lowercase();
    let users = online_users_state.get();
    let my_id = user_state.get_untracked().user_id.clone();
    let filtered: Vec<_> = users.users.iter()
      .filter(|u| u.user_id != my_id)
      .filter(|u| query.is_empty() || u.username.to_lowercase().contains(&query))
      .take(8) // Show max 8 items
      .cloned()
      .collect();
    let selected_idx = mention_selected_index.get().min(filtered.len().saturating_sub(1));

    if filtered.is_empty() {
      show_mention_list.set(false);
      return view! { <div class="mention-list-hidden"></div> }.into_any();
    }

    view! {
      <div class="mention-dropdown">
        <div class="mention-dropdown-header">{t!(i18n, chat_select_user)}</div>
        {filtered.into_iter().enumerate().map(|(idx, user)| {
          let username = user.username.clone();
          let username_click = username.clone();
          let username_aria = username.clone();
          let username_avatar = username.clone();
          let is_selected = idx == selected_idx;
          let item_class = if is_selected {
            "mention-dropdown-item selected"
          } else {
            "mention-dropdown-item"
          };
          let aria_label = format!("{} {}", t_string!(i18n, chat_mention_prefix), username_aria);
          view! {
            <div
              class=item_class
              tabindex=0
              aria-label=aria_label
              on:mousedown=move |ev: web_sys::MouseEvent| {
                ev.prevent_default(); // Prevent textarea from losing focus
                let new_text = insert_mention(&username_click, &input_text.get(), &mention_query.get());
                input_text.set(new_text);
                show_mention_list.set(false);
              }
            >
              <Avatar username=username_avatar size=AvatarSize::Small />
              <span class="mention-dropdown-name">{username}</span>
            </div>
          }
        }).collect_view()}
      </div>
    }.into_any()
  }
}
