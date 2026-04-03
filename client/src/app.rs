//! Application root component

use leptos::prelude::*;
use leptos_router::{
  components::{Route, Router, Routes},
  path,
};

use crate::{components, pages, services, state, storage, transfer, utils};

/// Application root component
#[component]
pub fn App() -> impl IntoView {
  // Initialize global state
  state::provide_app_state();

  // Initialize i18n internationalization
  crate::i18n::provide_i18n_context_with_persistence();

  // Initialize network services
  let ws_url = utils::get_ws_url();
  services::ws::WsClient::provide(&ws_url);
  services::webrtc::PeerManager::provide();
  crate::vad::VadManager::provide();
  crate::pip::PipManager::provide();
  crate::flow_control::FlowController::provide();
  transfer::TransferManager::provide();

  // Initialize theme
  init_theme();

  // Restore persisted conversations and messages from IndexedDB
  let chat_state = state::use_chat_state();
  storage::restore_from_db(chat_state);

  view! {
    <Router>
      <div class="app-container">
        // Toast notification container
        <components::ToastContainer />
        // Global modal manager (invite modal, incoming call modal, etc.)
        <components::ModalManager />

        <Routes fallback=|| view! { <pages::NotFound /> }>
          <Route path=path!("/login") view=pages::Login />
          <Route path=path!("/") view=pages::Home />
          <Route path=path!("/chat/:id") view=pages::ChatView />
          <Route path=path!("/room/:id") view=pages::RoomView />
          <Route path=path!("/theater/:id") view=pages::TheaterView />
          <Route path=path!("/settings") view=pages::Settings />
        </Routes>
      </div>
    </Router>
  }
}

/// Initialize theme: read preference from localStorage and apply to DOM
fn init_theme() {
  let theme_state = state::use_theme_state();

  // Read theme preference from localStorage
  if let Some(window) = web_sys::window()
    && let Ok(Some(storage)) = window.local_storage()
    && let Ok(Some(saved_theme)) = storage.get_item("theme")
  {
    let theme = match saved_theme.as_str() {
      "light" => state::Theme::Light,
      "dark" => state::Theme::Dark,
      _ => state::Theme::System,
    };
    theme_state.update(|s| s.theme = theme);
  }

  // Listen for theme changes and apply to DOM
  Effect::new(move |_| {
    let theme = theme_state.get().theme;
    if let Some(document) = web_sys::window().and_then(|w| w.document())
      && let Some(root) = document.document_element()
    {
      match theme {
        state::Theme::Light => {
          let _ = root.set_attribute("data-theme", "light");
        }
        state::Theme::Dark => {
          let _ = root.set_attribute("data-theme", "dark");
        }
        state::Theme::System => {
          let _ = root.remove_attribute("data-theme");
        }
      }
    }

    // Persist to localStorage
    if let Some(window) = web_sys::window()
      && let Ok(Some(storage)) = window.local_storage()
    {
      let value = match theme {
        state::Theme::Light => "light",
        state::Theme::Dark => "dark",
        state::Theme::System => "system",
      };
      let _ = storage.set_item("theme", value);
    }
  });
}
