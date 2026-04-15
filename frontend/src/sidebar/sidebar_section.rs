//! Sidebar section component.

use super::sidebar_conversation_item::SidebarConversationItem;
use leptos::prelude::*;

/// Sidebar section component.
#[component]
pub fn SidebarSection(
  #[prop(into)] title: Signal<String>,
  conversations: Signal<Vec<crate::state::Conversation>>,
) -> impl IntoView {
  view! {
    <div class="sidebar-section">
      <div class="sidebar-section-title">{title}</div>
      <For
        each=move || conversations.get()
        key=|conv| conv.id.clone()
        children=move |conv| {
          view! { <SidebarConversationItem conversation=conv.clone() /> }
        }
      />
    </div>
  }
}
