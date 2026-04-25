//! Sidebar navigation component.

mod sidebar_conversation_item;
mod sidebar_section;

use crate::i18n;
use crate::state::use_app_state;
use icondata as i;
use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use leptos_icons::Icon;
use sidebar_section::SidebarSection;

/// Sidebar navigation component.
///
/// Always visible on all viewport sizes. At narrow widths (`<768px`) it
/// collapses to an icon-only rail. Container queries inside the CSS drive
/// the responsive behavior -- no `hidden` class here.
#[component]
pub fn Sidebar() -> impl IntoView {
  let app_state = use_app_state();
  let i18n = i18n::use_i18n();

  view! {
    <aside class="sidebar" data-testid="sidebar">
      // Header: app title + logo
      <div class="sidebar-header">
        <div class="sidebar-brand">
          <Icon icon=i::LuMessageCircle attr:class="sidebar-brand-icon" />
          <span class="sidebar-brand-title">{t!(i18n, app.title)}</span>
        </div>
      </div>

      // Search
      <div class="sidebar-search">
        <Icon icon=i::LuSearch attr:class="sidebar-search-icon" />
        <input
          type="search"
          class="sidebar-search-input"
          placeholder=move || t_string!(i18n, common.search)
          aria-label=move || t_string!(i18n, common.search)
        />
      </div>

      // Conversation lists -- scrollable middle region
      <div class="sidebar-scroll">
        <SidebarSection
          title=move || t_string!(i18n, sidebar.pinned)
          conversations=Signal::derive(move || app_state.pinned_conversations())
        />
        <SidebarSection
          title=move || t_string!(i18n, sidebar.active)
          conversations=Signal::derive(move || app_state.active_conversations())
        />
        <SidebarSection
          title=move || t_string!(i18n, sidebar.archived)
          conversations=Signal::derive(move || app_state.archived_conversations())
        />
      </div>

      // Footer: settings gear, pinned to bottom
      <div class="sidebar-footer">
        <button
          class="sidebar-footer-btn"
          aria-label=move || t_string!(i18n, settings.title)
          title=move || t_string!(i18n, settings.title)
          on:click=move |_| app_state.settings_open.set(true)
          data-testid="sidebar-settings-btn"
        >
          <Icon icon=i::LuSettings attr:class="sidebar-footer-icon" />
          <span class="sidebar-footer-label">{t!(i18n, settings.title)}</span>
        </button>
      </div>
    </aside>
  }
}
