//! Sidebar navigation component.

mod sidebar_connection_status;
mod sidebar_conversation_item;
mod sidebar_conversation_menu;
mod sidebar_room_section;
mod sidebar_section;

use crate::components::discovery::OnlineUsersPanel;
use crate::i18n;
use crate::state::{Conversation, use_app_state};
use icondata as i;
use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use leptos_icons::Icon;
use sidebar_connection_status::SidebarConnectionStatus;
use sidebar_room_section::SidebarRoomSection;
use sidebar_section::SidebarSection;

/// Apply the sidebar search query against a conversation list.
///
/// Pure function so the case-insensitive substring filter can be
/// unit-tested without a Leptos runtime. The empty / whitespace-only
/// query is treated as "no filter" — every conversation passes
/// through unchanged.
#[must_use]
pub fn filter_conversations_by_query(
  conversations: Vec<Conversation>,
  query: &str,
) -> Vec<Conversation> {
  let q = query.trim().to_lowercase();
  if q.is_empty() {
    return conversations;
  }
  conversations
    .into_iter()
    .filter(|c| c.display_name.to_lowercase().contains(&q))
    .collect()
}

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

  // Memoised section views. Reading the conversation list directly
  // would re-filter and re-sort on every reactive update — three times
  // (pinned/active/archived) per render. The `*_memo` accessors cache
  // the result and only recompute when `app_state.conversations`
  // actually changes (Req 7.7 — performance follow-up from Task 24).
  let pinned_memo = app_state.pinned_conversations_memo();
  let active_memo = app_state.active_conversations_memo();
  let archived_memo = app_state.archived_conversations_memo();

  // Sidebar search query (G20). Drives a case-insensitive substring
  // filter over `display_name` for every section. The query lives in
  // local component state because it has no reach beyond this view —
  // promoting it to AppState would only make sense if a second
  // surface (settings, top-bar) ever needed to read it.
  let search_query = RwSignal::new(String::new());

  let pinned_filtered = Signal::derive(move || {
    filter_conversations_by_query(pinned_memo.get(), &search_query.get())
  });
  let active_filtered = Signal::derive(move || {
    filter_conversations_by_query(active_memo.get(), &search_query.get())
  });
  let archived_filtered = Signal::derive(move || {
    filter_conversations_by_query(archived_memo.get(), &search_query.get())
  });

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

      // Search — wired in G20: filters all three conversation
      // sections by case-insensitive substring on `display_name`.
      <div class="sidebar-search">
        <Icon icon=i::LuSearch attr:class="sidebar-search-icon" />
        <input
          type="search"
          class="sidebar-search-input"
          placeholder=move || t_string!(i18n, common.search)
          aria-label=move || t_string!(i18n, common.search)
          data-testid="sidebar-search-input"
          prop:value=move || search_query.get()
          on:input=move |ev| search_query.set(event_target_value(&ev))
        />
      </div>

      // Conversation lists -- scrollable middle region
      <div class="sidebar-scroll">
        <SidebarSection
          title=move || t_string!(i18n, sidebar.pinned)
          conversations=pinned_filtered
          kind="pinned"
        />
        <SidebarSection
          title=move || t_string!(i18n, sidebar.active)
          conversations=active_filtered
          kind="active"
        />
        <SidebarSection
          title=move || t_string!(i18n, sidebar.archived)
          conversations=archived_filtered
          kind="archived"
          collapsible=true
          expanded=app_state.archived_expanded
        />

        // Room list: browse/join/create rooms
        <SidebarRoomSection />

        // Discovery: online users + invite entry point (Req 9.1).
        // The user-info card opened by clicking a row is hosted
        // globally inside `ModalManager`; OnlineUsersPanel writes to
        // `GlobalRoomModalState::user_info_target` to open it.
        <OnlineUsersPanel />
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

#[cfg(test)]
mod tests {
  use super::filter_conversations_by_query;
  use crate::state::{Conversation, ConversationId, ConversationType};
  use message::types::UserId;

  fn conv(id: u64, name: &str) -> Conversation {
    Conversation {
      id: ConversationId::Direct(UserId::from(id)),
      display_name: name.to_string(),
      last_message: None,
      last_message_ts: None,
      unread_count: 0,
      pinned: false,
      pinned_ts: None,
      muted: false,
      archived: false,
      conversation_type: ConversationType::Direct,
    }
  }

  #[test]
  fn empty_query_returns_full_list() {
    let list = vec![conv(1, "Alice"), conv(2, "Bob")];
    let filtered = filter_conversations_by_query(list.clone(), "");
    assert_eq!(filtered.len(), 2);
    let filtered_ws = filter_conversations_by_query(list, "   ");
    assert_eq!(filtered_ws.len(), 2);
  }

  #[test]
  fn substring_match_is_case_insensitive() {
    let list = vec![conv(1, "Alice"), conv(2, "BOBBY"), conv(3, "Charlie")];
    let filtered = filter_conversations_by_query(list, "BO");
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].display_name, "BOBBY");
  }

  #[test]
  fn no_match_returns_empty_list() {
    let list = vec![conv(1, "Alice"), conv(2, "Bob")];
    let filtered = filter_conversations_by_query(list, "zzz");
    assert!(filtered.is_empty());
  }
}
