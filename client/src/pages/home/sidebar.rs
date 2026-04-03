//! Sidebar component

use leptos::prelude::*;
use leptos_i18n::{t, t_string};

use crate::{i18n::*, state};

use super::chat_list::ChatList;
use super::online_user_list::OnlineUserList;
use super::room_list::RoomList;
use super::sidebar_header::SidebarHeader;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SidebarTab {
  Chats,
  Users,
  Rooms,
}

/// Sidebar
#[component]
pub(super) fn Sidebar() -> impl IntoView {
  let active_tab = RwSignal::new(SidebarTab::Chats);
  let search_state = state::use_search_state();
  let chat_state = state::use_chat_state();
  let global_search_query = RwSignal::new(String::new());
  let i18n = use_i18n();

  view! {
    <div class="sidebar">
      // User info header
      <SidebarHeader />

      // Global message search
      <div class="sidebar-search">
        <div class="sidebar-search-wrap">
          <span class="sidebar-search-icon">"🔍"</span>
          <input
            class="sidebar-search-input"
            type="text"
            placeholder=move || t_string!(i18n, chat_search_all_messages)
            prop:value=move || global_search_query.get()
            on:input=move |ev| {
              let val = event_target_value(&ev);
              global_search_query.set(val.clone());
              search_state.update(|s| {
                s.query.clone_from(&val);
                s.show_panel = !val.is_empty();
              });
          // Debounced search: direct trigger (simple implementation)
              crate::storage::search_messages_async(search_state, chat_state, val);
            }
            on:focus=move |_| {
              if !global_search_query.get_untracked().is_empty() {
                search_state.update(|s| s.show_panel = true);
              }
            }
            on:keydown=move |ev: web_sys::KeyboardEvent| {
              if ev.key() == "Escape" {
                global_search_query.set(String::new());
                search_state.update(|s| {
                  s.query.clear();
                  s.results.clear();
                  s.show_panel = false;
                });
              }
            }
          />
          {move || {
            if global_search_query.get().is_empty() {
              view! { <span></span> }.into_any()
            } else {
              view! {
                <button
                  class="sidebar-search-clear"
                  on:click=move |_| {
                    global_search_query.set(String::new());
                    search_state.update(|s| {
                      s.query.clear();
                      s.results.clear();
                      s.show_panel = false;
                    });
                  }
                >"✕"</button>
              }.into_any()
            }
          }}
        </div>
      </div>

      // Global search results panel
      {move || {
        let s = search_state.get();
        if !s.show_panel {
          return view! { <div class="search-panel-hidden"></div> }.into_any();
        }

        if s.is_searching {
          return view! {
            <div class="search-results-panel">
              <div class="search-results-loading">{t!(i18n, common_searching)}</div>
            </div>
          }.into_any();
        }

        if s.results.is_empty() && !s.query.is_empty() {
          return view! {
            <div class="search-results-panel">
              <crate::components::EmptyState
                icon="🔍"
                title=t_string!(i18n, common_no_results).to_string()
                description=t_string!(i18n, common_try_other_keywords).to_string()
              />
            </div>
          }.into_any();
        }

        view! {
          <div class="search-results-panel">
            <div class="search-results-header">
              <span class="search-results-title">{format!("{}", t_string!(i18n, chat_search_found_results).replace("{}", &s.results.len().to_string()))}</span>
            </div>
            <div class="search-results-list">
              {s.results.iter().map(|item| {
                let conv_name = item.conversation_name.clone();
                let from = item.from.clone();
                let preview = item.preview.clone();
                let ts = item.timestamp;
                let _conv_id = item.conversation_id.clone();
                let _msg_id = item.message_id.clone();
                // Format time
                let time_str = {
                  let date = js_sys::Date::new_0();
                  date.set_time(ts as f64 * 1000.0);
                  let h = date.get_hours();
                  let m = date.get_minutes();
                  let month = date.get_month() + 1;
                  let day = date.get_date();
                  format!("{month:02}/{day:02} {h:02}:{m:02}")
                };
                view! {
                  <div class="search-result-item" tabindex=0>
                    <div class="search-result-header">
                      <span class="search-result-conv">{conv_name}</span>
                      <span class="search-result-time">{time_str}</span>
                    </div>
                    <div class="search-result-body">
                      <span class="search-result-from">{format!("{from}:")}</span>
                      <span class="search-result-preview">{preview}</span>
                    </div>
                  </div>
                }
              }).collect_view()}
            </div>
          </div>
        }.into_any()
      }}

      // Tab switching
      <div class="sidebar-tabs">
        <button
          class=move || format!("sidebar-tab {}", if active_tab.get() == SidebarTab::Chats { "active" } else { "" })
          on:click=move |_| active_tab.set(SidebarTab::Chats)
          tabindex=0
          aria-label=move || t_string!(i18n, nav_chat)
        >
          "💬 " {t!(i18n, nav_chat)}
        </button>
        <button
          class=move || format!("sidebar-tab {}", if active_tab.get() == SidebarTab::Users { "active" } else { "" })
          on:click=move |_| active_tab.set(SidebarTab::Users)
          tabindex=0
          aria-label=move || t_string!(i18n, nav_online_users)
        >
          "👥 " {t!(i18n, nav_online_users)}
        </button>
        <button
          class=move || format!("sidebar-tab {}", if active_tab.get() == SidebarTab::Rooms { "active" } else { "" })
          on:click=move |_| active_tab.set(SidebarTab::Rooms)
          tabindex=0
          aria-label=move || t_string!(i18n, nav_rooms)
        >
          "🏠 " {t!(i18n, nav_rooms)}
        </button>
      </div>

      // Tab content
      <div class="sidebar-content">
        {move || match active_tab.get() {
          SidebarTab::Chats => view! { <ChatList /> }.into_any(),
          SidebarTab::Users => view! { <OnlineUserList /> }.into_any(),
          SidebarTab::Rooms => view! { <RoomList /> }.into_any(),
        }}
      </div>
    </div>
  }
}
