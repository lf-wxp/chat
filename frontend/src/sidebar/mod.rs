//! Sidebar navigation component.

mod sidebar_conversation_item;
mod sidebar_section;

use crate::i18n;
use crate::state::use_app_state;
use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use sidebar_section::SidebarSection;

/// Sidebar navigation component.
#[component]
pub fn Sidebar() -> impl IntoView {
  let app_state = use_app_state();
  let i18n = i18n::use_i18n();

  view! {
    <aside class="sidebar w-64 flex-col border-r hidden md:flex" data-testid="sidebar">
      // Sidebar Header
      <div class="sidebar-header">
        <span class="sidebar-header-title">{t!(i18n, app.title)}</span>
        // Settings button
        <button
          aria-label=move || t_string!(i18n, settings.title)
          class="sidebar-item"
        >
          <span class="sidebar-item-icon">"gear"</span>
        </button>
      </div>

      // Search
      <div class="px-4 py-2">
        <input
          type="search"
          class="input input-sm"
          placeholder=move || t_string!(i18n, common.search)
          aria-label=move || t_string!(i18n, common.search)
        />
      </div>

      // Pinned Conversations
      <SidebarSection
        title=move || t_string!(i18n, sidebar.pinned)
        conversations=Signal::derive(move || app_state.pinned_conversations())
      />

      // Active Conversations
      <SidebarSection
        title=move || t_string!(i18n, sidebar.active)
        conversations=Signal::derive(move || app_state.active_conversations())
      />

      // Archived Conversations
      <SidebarSection
        title=move || t_string!(i18n, sidebar.archived)
        conversations=Signal::derive(move || app_state.archived_conversations())
      />
    </aside>

    // Mobile Sidebar Overlay
    <aside class="sidebar sidebar-collapsed md:hidden" data-testid="mobile-sidebar">
      // Controlled by mobile hamburger menu
    </aside>
  }
}
