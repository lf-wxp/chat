//! Main content area header component

use leptos::prelude::*;
use leptos_i18n::t_string;
use message::signal::{InviteType, SignalMessage};

use crate::{
  components::{Button, ButtonVariant},
  i18n::*,
  services::ws::WsClient,
  state,
};

use super::invite_link_panel::InviteLinkPanel;

/// Main content area header
#[component]
pub(super) fn MainHeader() -> impl IntoView {
  let ui_state = state::use_ui_state();
  let i18n = use_i18n();

  view! {
    <header class="main-header">
      // Mobile menu button
      <button
        class="menu-btn mobile-only"
        on:click=move |_| ui_state.update(|s| s.sidebar_open = !s.sidebar_open)
        tabindex=0
        aria-label=move || t_string!(i18n, nav_open_menu)
      >
        "☰"
      </button>
      <h2 class="main-header-title">{t_string!(i18n, app_title)}</h2>
      <div class="main-header-actions">
        <Button
          label=t_string!(i18n, invite_generate_link).to_string()
          variant=ButtonVariant::Ghost
          on_click=Callback::new(|()| {
            let ws = WsClient::use_client();
            let _ = ws.send(&SignalMessage::CreateInviteLink {
              invite_type: InviteType::Chat,
              room_id: None,
            });
          })
        />
      </div>
    </header>
    // Invite link panel
    <InviteLinkPanel />
  }
}
