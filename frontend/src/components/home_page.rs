//! Home page component.
//!
//! Composes the root chat surface: when a conversation is selected
//! `<ChatView />` renders the Task 16 chat pane, otherwise a welcome
//! placeholder is shown.

use crate::components::chat_view::ChatView;
use crate::i18n;
use crate::state::use_app_state;
use leptos::prelude::*;
use leptos_i18n::t;

/// Home page component.
#[component]
pub fn HomePage() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let app_state = use_app_state();
  let has_conversation = Memo::new(move |_| app_state.active_conversation.get().is_some());

  view! {
    <Show
      when=move || has_conversation.get()
      fallback=move || view! {
        <div class="flex items-center justify-center h-full">
          <div class="text-center p-8">
            <h1 class="text-2xl font-bold mb-4">{t!(i18n, app.title)}</h1>
            <p class="text-secondary">{t!(i18n, app.welcome)}</p>
          </div>
        </div>
      }
    >
      <ChatView />
    </Show>
  }
}
