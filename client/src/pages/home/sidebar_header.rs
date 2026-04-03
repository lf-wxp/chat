//! Sidebar header (user info + status selection)

use leptos::prelude::*;
use leptos_i18n::t_string;
use message::signal::{SignalMessage, UserStatus};

use crate::{
  components::{Avatar, AvatarSize},
  i18n::*,
  services::ws::WsClient,
  state,
};

/// Display text for each status
fn status_text(status: UserStatus) -> &'static str {
  match status {
    UserStatus::Online => "online",
    UserStatus::Busy => "busy",
    UserStatus::Away => "away",
    UserStatus::Offline => "offline",
  }
}

/// Emoji icon for each status
fn status_icon(status: UserStatus) -> &'static str {
  match status {
    UserStatus::Online => "🟢",
    UserStatus::Busy => "🔴",
    UserStatus::Away => "🟡",
    UserStatus::Offline => "⚫",
  }
}

/// CSS class for each status
fn status_dot_class(status: UserStatus) -> &'static str {
  match status {
    UserStatus::Online => "status-dot-online",
    UserStatus::Busy => "status-dot-busy",
    UserStatus::Away => "status-dot-away",
    UserStatus::Offline => "status-dot-offline",
  }
}

/// Sidebar header (user info + status selection)
#[component]
pub(super) fn SidebarHeader() -> impl IntoView {
  let user_state = state::use_user_state();
  let show_status_menu = RwSignal::new(false);
  let i18n = use_i18n();

  // Callback for switching status
  let set_status = move |new_status: UserStatus| {
    // Update local state
    user_state.update(|s| s.status = new_status);
    // Notify server via WebSocket
    let my_id = user_state.get_untracked().user_id.clone();
    let ws = WsClient::use_client();
    let _ = ws.send(&SignalMessage::UserStatusChange {
      user_id: my_id,
      status: new_status,
    });
    show_status_menu.set(false);
  };

  let username_for_avatar = user_state.get_untracked().username.clone();

  view! {
    <div class="sidebar-header">
      <div class="sidebar-header-user">
        // Avatar + status indicator dot
        <div
          class="sidebar-avatar-wrap"
          on:click=move |_| show_status_menu.update(|v| *v = !*v)
          title=move || t_string!(i18n, common_click_to_switch_status)
          style="cursor:pointer;position:relative;"
        >
          <Avatar
            username=username_for_avatar.clone()
            size=AvatarSize::Small
            online=false
          />
          <span class=move || {
            format!("sidebar-status-dot {}", status_dot_class(user_state.get().status))
          }></span>
        </div>

        // Username + current status text
        <div class="flex-1" style="min-width:0;">
          <div class="font-medium truncate">{move || user_state.get().username.clone()}</div>
          <div
            class="sidebar-status-text"
            on:click=move |_| show_status_menu.update(|v| *v = !*v)
            style="cursor:pointer;"
          >
            <span class="sidebar-status-label">
              {move || {
                let s = user_state.get().status;
                let text = match s {
                  UserStatus::Online => t_string!(i18n, common_online),
                  UserStatus::Busy => t_string!(i18n, common_busy),
                  UserStatus::Away => t_string!(i18n, common_away),
                  UserStatus::Offline => t_string!(i18n, common_offline),
                };
                format!("{} {}", status_icon(s), text)
              }}
            </span>
            <span class="sidebar-status-arrow">"▾"</span>
          </div>
        </div>
      </div>

      // Status selection dropdown menu
      {move || {
        if !show_status_menu.get() {
          return view! { <div class="status-menu-hidden"></div> }.into_any();
        }
        let current = user_state.get().status;
        let statuses = [
          UserStatus::Online,
          UserStatus::Busy,
          UserStatus::Away,
          UserStatus::Offline,
        ];
        view! {
          <div class="status-menu-backdrop" on:click=move |_| show_status_menu.set(false)>
            <div
              class="status-menu"
              on:click=move |ev: web_sys::MouseEvent| ev.stop_propagation()
            >
              <div class="status-menu-title">{t!(i18n, common_click_to_switch_status)}</div>
              {statuses.iter().map(|&s| {
                let is_active = s == current;
                let item_class = if is_active { "status-menu-item active" } else { "status-menu-item" };
                view! {
                  <button
                    class=item_class
                    on:click=move |_| set_status(s)
                  >
                    <span class="status-menu-icon">{status_icon(s)}</span>
                    <span class="status-menu-label">{match s {
                      UserStatus::Online => t_string!(i18n, common_online).to_string(),
                      UserStatus::Busy => t_string!(i18n, common_busy).to_string(),
                      UserStatus::Away => t_string!(i18n, common_away).to_string(),
                      UserStatus::Offline => t_string!(i18n, common_offline).to_string(),
                    }}</span>
                    {if is_active {
                      view! { <span class="status-menu-check">"✓"</span> }.into_any()
                    } else {
                      ().into_any()
                    }}
                  </button>
                }
              }).collect_view()}
            </div>
          </div>
        }.into_any()
      }}
    </div>
  }
}
