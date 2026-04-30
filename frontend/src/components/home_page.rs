//! Home page component.
//!
//! Renders the active chat view. When no conversation is selected,
//! a welcoming empty state is shown prompting the user to select
//! a conversation from the sidebar or join a room.

use crate::components::chat_view::ChatView;
use crate::i18n;
use crate::state::use_app_state;
use icondata as i;
use leptos::prelude::*;
use leptos_i18n::t_string;
use leptos_icons::Icon;

/// Home page component.
#[component]
pub fn HomePage() -> impl IntoView {
  let app_state = use_app_state();
  let i18n = i18n::use_i18n();
  let has_conversation = Memo::new(move |_| app_state.active_conversation.get().is_some());

  view! {
    <Show when=move || has_conversation.get()>
      <ChatView />
    </Show>
    <Show when=move || !has_conversation.get()>
      <div class="home-empty" data-testid="home-empty">
        <div class="home-empty__icon"><Icon icon=i::LuMessageSquare /></div>
        <h2 class="home-empty__title">{t_string!(i18n, app.title)}</h2>
        <p class="home-empty__hint">{t_string!(i18n, home.select_conversation)}</p>
      </div>
    </Show>
  }
}
