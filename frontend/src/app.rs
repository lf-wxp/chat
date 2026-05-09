//! Root App component.
//!
//! Provides the main layout structure including sidebar, chat area,
//! and overlay components. Handles theme switching and responsive layout.

use crate::components::{
  AppBg, AuthPage, CallOverlay, DebugPanel, ErrorToastContainer, GlobalRoomModalState, HomePage,
  IncomingInviteModal, ModalManager, OfflineBanner, PwaInstallPrompt, PwaUpdateBanner,
  ReconnectBanner, SettingsPage, Sidebar, ToastContainer, TopBar,
};
use crate::i18n::{self, Locale};
use crate::i18n_helpers;
use crate::logging::use_logger_state;
use crate::settings::use_settings_state;
use crate::state::use_app_state;
use crate::utils;
use leptos::prelude::*;
use leptos_i18n::t_string;
use leptos_use::use_event_listener;

/// Root App component.
#[component]
pub fn App() -> impl IntoView {
  let app_state = use_app_state();
  let logger = use_logger_state();
  let i18n = i18n::use_i18n();

  // Install the Debug Panel visibility signal at the App root so
  // sibling components (Settings drawer's "Open Debug Panel" button,
  // header shortcuts, etc.) can toggle it via context. Previously
  // the signal was provided inside `DebugPanel` itself, which made
  // it invisible to siblings like `SettingsPage` (V2-S-3 fix).
  let _ = crate::components::debug::provide_debug_panel_visibility();

  // Reactive signal tracking whether the system prefers dark mode.
  // `use_media_query` automatically subscribes to changes and cleans
  // up on unmount — no manual `Closure` / `StoredValue` / `on_cleanup`
  // bookkeeping required.
  let prefers_dark = leptos_use::use_media_query("(prefers-color-scheme: dark)");

  // Theme switching effect. Bumps the shared `saved_tick` on every
  // user-initiated change so the Settings drawer's "Saved" indicator
  // fires for theme toggles as well (Req 13.6.3 — V2-S-1). The initial
  // Effect run (which only reflects the persisted value) is skipped
  // via a previous-value guard provided by the `Effect` closure's
  // `prev` parameter.
  let theme = app_state.theme;
  let settings_for_theme = use_settings_state();
  Effect::new(move |prev: Option<String>| {
    // Read the reactive signal so this effect re-runs when the
    // system color-scheme preference changes.
    let is_dark = prefers_dark.get();
    let theme_val = theme.get();
    if let Some(window) = web_sys::window()
      && let Some(document) = window.document()
      && let Some(html) = document.document_element()
    {
      let resolved_theme = match theme_val.as_str() {
        "dark" => "dark",
        "light" => "light",
        _ => {
          // "system" -- use the reactive prefers-dark signal
          if is_dark { "dark" } else { "light" }
        }
      };
      let _ = html.set_attribute("data-theme", resolved_theme);
      // Persist theme preference to localStorage under the namespaced
      // `settings_theme` key (Req 13 — unified `settings_` prefix).
      utils::save_to_local_storage("settings_theme", &theme_val);
    }
    // Skip the very first run so we don't flash "Saved" on startup.
    if let Some(previous) = &prev
      && previous != &theme_val
    {
      settings_for_theme.bump_saved();
    }
    theme_val
  });

  // Locale switching effect. Also bumps `saved_tick` so the
  // "Saved" indicator fires for language changes (V2-S-1).
  let locale = app_state.locale;
  let settings_for_locale = use_settings_state();
  Effect::new(move |prev: Option<String>| {
    let locale_val = locale.get();
    let new_locale = if locale_val.starts_with("zh") {
      Locale::zh_CN
    } else if locale_val.starts_with("es") {
      Locale::es
    } else {
      Locale::en
    };
    i18n.set_locale(new_locale);
    i18n_helpers::persist_locale(new_locale);
    if let Some(previous) = &prev
      && previous != &locale_val
    {
      settings_for_locale.bump_saved();
    }
    locale_val
  });

  // Debug mode effect -- adjust logging verbosity
  let debug = app_state.debug;
  Effect::new(move || {
    logger.set_debug_mode(debug.get());
  });

  // Font scale effect -- mirror the chosen FontScale to
  // `<html data-font-scale="...">` so the CSS tokens layer can pick
  // it up via the `[data-font-scale]` selector.
  let settings_state = use_settings_state();
  Effect::new(move || {
    let scale = settings_state.get().font_scale;
    if let Some(window) = web_sys::window()
      && let Some(document) = window.document()
      && let Some(html) = document.document_element()
    {
      let _ = html.set_attribute("data-font-scale", scale.as_str());
    }
  });

  // Visual-effect toggles (MVP batch 8b). Mirrors the Appearance
  // switches onto `<html data-glass="on|off">` and
  // `<html data-motion="on|off">` so the effects layer CSS (glass,
  // animations) can disable itself without each stylesheet needing
  // to know about settings state.
  let settings_visual = use_settings_state();
  Effect::new(move || {
    let settings = settings_visual.get();
    if let Some(window) = web_sys::window()
      && let Some(document) = window.document()
      && let Some(html) = document.document_element()
    {
      let glass_value = if settings.glass_enabled { "on" } else { "off" };
      let motion_value = if settings.motion_enabled { "on" } else { "off" };
      let _ = html.set_attribute("data-glass", glass_value);
      let _ = html.set_attribute("data-motion", motion_value);
    }
  });

  // Auth gate: show auth page when not authenticated, main app otherwise
  let auth = app_state.auth;

  // Provide global room modal context so that modals are rendered at the
  // app root (inside ModalManager) instead of being clipped by sidebar
  // overflow.
  let _room_modal_state = GlobalRoomModalState::provide();

  // Global Escape handler (Req 14.5.2). Closes the top-most overlay
  // in priority order: settings drawer → sidebar overlay. Child
  // components (modals, menus) manage their own Escape handlers so
  // they continue to win when focus is inside them — the event
  // bubbles up to window here only if no inner handler intercepted.
  let settings_open = app_state.settings_open;
  let sidebar_visible = app_state.sidebar_visible;
  let _ = use_event_listener(
    leptos_use::use_window(),
    leptos::ev::keydown,
    move |ev: web_sys::KeyboardEvent| {
      if utils::safe_key(&ev) != "Escape" {
        return;
      }
      if settings_open.get_untracked() {
        settings_open.set(false);
        return;
      }
      // Mobile-only: collapse the sidebar overlay.
      if let Some(window) = web_sys::window()
        && let Some(doc) = window.document()
      {
        let width = doc
          .document_element()
          .map(|el| el.client_width())
          .unwrap_or(0);
        if width > 0 && width < 768 && sidebar_visible.get_untracked() {
          sidebar_visible.set(false);
        }
      }
    },
  );

  view! {
      // Decorative full-viewport background. Sits behind every shell
      // (including the auth gate) so the themed gradient is always
      // visible. Presentation-only; no reactive state.
      <AppBg />

      // Global overlays sit outside the auth gate so error toasts and
      // the reconnect banner remain visible on the login/register page
      // (Code Quality 1 fix).
      <ErrorToastContainer />
      <ReconnectBanner />
      <OfflineBanner />
      <PwaUpdateBanner />
      <PwaInstallPrompt />

      // Skip-to-content link (WCAG 2.4.1, Req 14.6). Visually hidden
      // until focused; jumps to #main-content so keyboard-only users
      // can bypass the sidebar. The label is wrapped in a reactive
      // closure so it updates on locale switch (leptos_i18n requires
      // `t_string!` calls to live inside a reactive tracking context).
      <a class="skip-link" href="#main-content">
        {move || t_string!(i18n, a11y.skip_to_content)}
      </a>

      <Show
        when=move || auth.get().is_some()
        fallback=move || view! { <AuthPage /> }
      >
        <div class="app flex overflow-hidden">
          // Sidebar
          <Sidebar />

          // Main Content Area
          <main id="main-content" class="flex-1 flex flex-col min-w-0 overflow-hidden">
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
