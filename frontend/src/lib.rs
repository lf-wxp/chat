//! Frontend library entry point.
//!
//! Sets up Leptos WASM application with global state,
//! logging, i18n, and routing.

pub mod app;
pub mod chat_view;
pub mod config;
pub mod debug_log_entry;
pub mod debug_panel;
pub mod home_page;
pub mod i18n_helpers;
pub mod logging;
pub mod modal_manager;
pub mod reconnect_banner;
pub mod settings_page;
pub mod sidebar;
pub mod state;
pub mod toast_container;
pub mod top_bar;
pub mod utils;

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
    provide_app_state();
    provide_logger_state();

    view! {
      <i18n::I18nContextProvider>
        <app::App />
      </i18n::I18nContextProvider>
    }
  });
}
