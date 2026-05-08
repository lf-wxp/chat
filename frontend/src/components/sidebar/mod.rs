//! Sidebar navigation component.

mod sidebar_connection_status;
mod sidebar_conversation_item;
mod sidebar_conversation_menu;
mod sidebar_room_section;
mod sidebar_section;

use crate::components::discovery::{OnlineUsersPanel, UserInfoCard};
use crate::i18n;
use crate::state::use_app_state;
use icondata as i;
use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use leptos_icons::Icon;
use message::UserId;
use sidebar_connection_status::SidebarConnectionStatus;
use sidebar_room_section::SidebarRoomSection;
use sidebar_section::SidebarSection;

/// Sidebar navigation component.
///
/// On desktop (≥768px) the sidebar is always visible at 16rem width.
/// On mobile (<768px) the sidebar is hidden by default and shown as a
/// full-width overlay when the user taps the menu button in the top bar.
/// Selecting a conversation or pressing back closes the overlay.
#[component]
pub fn Sidebar() -> impl IntoView {
  let app_state = use_app_state();
  let i18n = i18n::use_i18n();

  // Selection state shared between the online-users panel (which sets
  // it on row click) and the user info card modal (which renders for
  // the selected user). Lives in the sidebar so it survives navigation
  // inside the chat area without remounting the modal.
  let selected_user = RwSignal::new(Option::<UserId>::None);

  // Memoised section views. Reading the conversation list directly
  // would re-filter and re-sort on every reactive update — three times
  // (pinned/active/archived) per render. The `*_memo` accessors cache
  // the result and only recompute when `app_state.conversations`
  // actually changes (Req 7.7 — performance follow-up from Task 24).
  let pinned_memo = app_state.pinned_conversations_memo();
  let active_memo = app_state.active_conversations_memo();
  let archived_memo = app_state.archived_conversations_memo();

  // On desktop the sidebar is always visible. On mobile, it is hidden
  // unless the user has explicitly opened it via the menu button.
  // When a conversation is selected the sidebar auto-closes.
  let sidebar_class = move || {
    let visible = app_state.sidebar_visible.get();
    if visible {
      "sidebar"
    } else {
      "sidebar sidebar-mobile-hidden"
    }
  };

  view! {
    // Mobile backdrop overlay — tapping it closes the sidebar
    <Show when=move || app_state.sidebar_visible.get()>
      <div
        class="sidebar-backdrop"
        on:click=move |_| app_state.sidebar_visible.set(false)
        data-testid="sidebar-backdrop"
      />
    </Show>
    <aside class=sidebar_class data-testid="sidebar">
      // Header: app title + logo + close button (mobile)
      <div class="sidebar-header">
        <div class="sidebar-brand">
          <Icon icon=i::LuMessageCircle attr:class="sidebar-brand-icon" />
          <span class="sidebar-brand-title">{t!(i18n, app.title)}</span>
          <SidebarConnectionStatus />
        </div>
        // Close button: mobile only, closes the sidebar overlay
        <button
          class="sidebar-close-btn"
          aria-label=move || t_string!(i18n, common.close)
          title=move || t_string!(i18n, common.close)
          on:click=move |_| app_state.sidebar_visible.set(false)
        >
          <Icon icon=i::LuX />
        </button>
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
          conversations=pinned_memo.into()
        />
        <SidebarSection
          title=move || t_string!(i18n, sidebar.active)
          conversations=active_memo.into()
        />
        <SidebarSection
          title=move || t_string!(i18n, sidebar.archived)
          conversations=archived_memo.into()
          collapsible=true
          expanded=app_state.archived_expanded
        />

        // Room list: browse/join/create rooms
        <SidebarRoomSection />

        // Discovery: online users + invite entry point (Req 9.1).
        <OnlineUsersPanel selected=selected_user />
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

      // User info card overlay (rendered while `selected_user` is Some).
      <UserInfoCard target=selected_user />
    </aside>
  }
}
