//! Home page (layout + sidebar + chat list + online users + room list)

mod chat_list;
mod invite_link_panel;
mod main_header;
mod online_user_list;
mod room_list;
mod sidebar;
mod sidebar_header;

use leptos::prelude::*;
use leptos_i18n::t_string;
use leptos_router::NavigateOptions;
use leptos_router::hooks::use_navigate;

use crate::{components::EmptyState, i18n::*, state};

use main_header::MainHeader;
use sidebar::Sidebar;

/// Home page
#[component]
pub fn Home() -> impl IntoView {
  let user_state = state::use_user_state();
  let ui_state = state::use_ui_state();
  let i18n = use_i18n();

  // Redirect to login page if not authenticated
  Effect::new(move |_| {
    if !user_state.get().authenticated {
      let navigate = use_navigate();
      navigate("/login", NavigateOptions::default());
    }
  });

  view! {
    <div class="layout-wrapper">
      // Mobile overlay
      <div
        class=move || format!("layout-overlay {}", if ui_state.get().sidebar_open { "visible" } else { "" })
        on:click=move |_| ui_state.update(|s| s.sidebar_open = false)
      ></div>

      // Sidebar
      <aside class=move || format!("layout-sidebar {}", if ui_state.get().sidebar_open { "open" } else { "" })>
        <Sidebar />
      </aside>

      // Main content area
      <div class="layout-main">
        <MainHeader />
        <div class="main-content">
          <EmptyState
            icon="💬"
            title=t_string!(i18n, chat_select_conversation).to_string()
            description=t_string!(i18n, chat_select_conversation_desc).to_string()
          />
        </div>
      </div>
    </div>
  }
}
