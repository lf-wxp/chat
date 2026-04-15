//! Chat view component (placeholder for Task 16).

use crate::i18n;
use leptos::prelude::*;
use leptos_i18n::t;

/// Chat view component (placeholder for Task 16).
#[component]
pub fn ChatView() -> impl IntoView {
  let i18n = i18n::use_i18n();
  view! {
    <div class="flex flex-col h-full" data-testid="chat-view">
      // Message list, input bar, etc. -- to be implemented in Task 16
      <div class="flex-1 flex items-center justify-center">
        <p class="text-tertiary">{t!(i18n, chat.type_message)}</p>
      </div>
    </div>
  }
}
