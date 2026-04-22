//! Frontend library entry point.
//!
//! Sets up Leptos WASM application with global state,
//! logging, i18n, and routing.

pub mod app;
pub mod auth;
pub mod chat_view;
pub mod config;
pub mod debug_log_entry;
pub mod debug_panel;
pub mod error_handler;
pub mod home_page;
pub mod i18n_helpers;
pub mod identicon;
pub mod logging;
pub mod modal_manager;
pub mod reconnect_banner;
pub mod settings_page;
pub mod sidebar;
pub mod signaling;
pub mod state;
pub mod toast_container;
pub mod top_bar;
pub mod user_status;
pub mod utils;
pub mod webrtc;

// Include the auto-generated i18n module from build.rs (leptos_i18n_build).
// This creates `pub mod i18n { ... }` with Locale, I18nContextProvider, use_i18n, t!, etc.
include!(concat!(env!("OUT_DIR"), "/mod.rs"));

use leptos::prelude::*;
use logging::provide_logger_state;
use state::provide_app_state;
use wasm_bindgen::prelude::wasm_bindgen;

/// Initialize and mount the Leptos application.
///
/// This function is automatically called by Trunk when the WASM module loads.
#[wasm_bindgen(start)]
pub fn init() {
  // Mount the App component wrapped with I18nContextProvider.
  // Global states are provided inside mount_to_body so they live under
  // the correct reactive owner created by the mount call.
  mount_to_body(|| {
    let app_state = provide_app_state();
    provide_logger_state();

    // Initialize config once and provide via context
    crate::config::provide_config();

    // Initialize error toast manager BEFORE signaling client so that
    // any ErrorResponse received during connection setup can be
    // displayed without a missing-context panic (P0 Bug-2 fix).
    let error_toast = error_handler::provide_error_toast_manager();

    // Initialize user status manager BEFORE signaling client so the
    // signaling client can cache a reference for use in WebSocket
    // callbacks (where Leptos context is unavailable).
    let user_status = user_status::provide_user_status_manager(app_state);

    // Initialize signaling client and provide via context
    let signaling =
      signaling::provide_signaling_client(app_state, user_status.clone(), error_toast);

    // Break circular dependency: UserStatusManager needs SignalingClient
    // for sending status messages from browser event callbacks.
    user_status.set_signaling_client(signaling.clone());

    // Cache LoggerState so browser event callbacks in UserStatusManager
    // can log through the structured logger instead of falling back to
    // console.warn (P1-3 fix).
    if let Some(logger) = leptos::prelude::use_context::<logging::LoggerState>() {
      user_status.set_logger(logger);
    }

    // Initialize WebRTC manager and provide via context
    let webrtc_manager = webrtc::provide_webrtc_manager(app_state);
    webrtc_manager.set_signaling_client(signaling);

    // Attempt to recover auth state from localStorage
    auth::try_recover_auth(app_state);

    view! {
      <i18n::I18nContextProvider>
        <app::App />
      </i18n::I18nContextProvider>
    }
  });
}
