//! Root App component.
//!
//! Provides the main layout structure including sidebar, chat area,
//! and overlay components. Handles theme switching and responsive layout.

use crate::components::{
  AuthPage, CallOverlay, DebugPanel, ErrorToastContainer, HomePage, IncomingInviteModal,
  ModalManager, ReconnectBanner, SettingsPage, Sidebar, ToastContainer, TopBar,
};
use crate::i18n::{self, Locale};
use crate::i18n_helpers;
use crate::logging::use_logger_state;
use crate::state::use_app_state;
use crate::utils;
use leptos::prelude::*;
use wasm_bindgen::prelude::*;

/// Root App component.
#[component]
pub fn App() -> impl IntoView {
  let app_state = use_app_state();
  let logger = use_logger_state();
  let i18n = i18n::use_i18n();

  // Dedicated trigger to force theme recalculation when the system
  // color scheme preference changes (avoids the theme.set("system") hack).
  let system_theme_trigger = Trigger::new();

  // Theme switching effect
  let theme = app_state.theme;
  Effect::new(move || {
    // Track the trigger so this effect re-runs on system theme changes
    system_theme_trigger.track();
    let theme_val = theme.get();
    if let Some(window) = web_sys::window()
      && let Some(document) = window.document()
      && let Some(html) = document.document_element()
    {
      let resolved_theme = match theme_val.as_str() {
        "dark" => "dark",
        "light" => "light",
        _ => {
          // "system" -- check prefers-color-scheme
          let has_dark_preference = window
            .match_media("(prefers-color-scheme: dark)")
            .ok()
            .flatten()
            .map(|mql| mql.matches())
            .unwrap_or(false);
          if has_dark_preference { "dark" } else { "light" }
        }
      };
      let _ = html.set_attribute("data-theme", resolved_theme);
      // Persist theme preference to localStorage
      utils::save_to_local_storage("theme", &theme_val);
    }
  });

  // Watch for system theme changes -- register listener with cleanup
  if let Some(window) = web_sys::window()
    && let Ok(Some(mql)) = window.match_media("(prefers-color-scheme: dark)")
  {
    let on_change = Closure::wrap(Box::new({
      let trigger = system_theme_trigger;
      move |_: web_sys::MediaQueryListEvent| {
        // Only re-trigger when the user preference is "system"
        if theme.get() == "system" {
          trigger.notify();
        }
      }
    }) as Box<dyn Fn(_)>);
    // Set the callback BEFORE converting closure to JsValue
    mql.set_onchange(Some(on_change.as_ref().unchecked_ref::<js_sys::Function>()));
    // Store closure in StoredValue to prevent GC; clean up on unmount
    let stored_closure = StoredValue::new(on_change.into_js_value());
    let mql_clone = mql.clone();
    on_cleanup(move || {
      // Clear the listener so the closure can be GC'd
      mql_clone.set_onchange(None);
      stored_closure.dispose();
    });
  }

  // Locale switching effect
  let locale = app_state.locale;
  Effect::new(move || {
    let locale_val = locale.get();
    let new_locale = if locale_val.starts_with("zh") {
      Locale::zh_CN
    } else {
      Locale::en
    };
    i18n.set_locale(new_locale);
    i18n_helpers::persist_locale(new_locale);
  });

  // Debug mode effect -- adjust logging verbosity
  let debug = app_state.debug;
  Effect::new(move || {
    logger.set_debug_mode(debug.get());
  });

  // Auth gate: show auth page when not authenticated, main app otherwise
  let auth = app_state.auth;

  view! {
    // Global overlays sit outside the auth gate so error toasts and
    // the reconnect banner remain visible on the login/register page
    // (Code Quality 1 fix).
    <ErrorToastContainer />
    <ReconnectBanner />

    <Show
      when=move || auth.get().is_some()
      fallback=move || view! { <AuthPage /> }
    >
      <div class="app flex overflow-hidden">
        // Sidebar
        <Sidebar />

        // Main Content Area
        <main class="flex-1 flex flex-col min-w-0 overflow-hidden">
          // Top Bar / Header
          <TopBar />

          // Chat Area
          <div class="flex-1 overflow-y-auto">
            <HomePage />
          </div>
        </main>

        // Settings drawer -- always mounted so its CSS transition can play
        <SettingsPage />

        // Overlays scoped to the authenticated shell
        <ToastContainer />
        <ModalManager />
        <CallOverlay />
        <IncomingInviteModal />
        <DebugPanel />
      </div>
    </Show>
  }
}
