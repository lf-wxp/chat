//! Frontend library entry point.
//!
//! Sets up Leptos WASM application with global state,
//! logging, i18n, and routing.

pub mod app;
pub mod auth;
pub mod blacklist;
pub mod call;
pub mod chat;
pub mod components;
pub mod config;
pub mod cross_tab;
pub mod error_handler;
pub mod file_transfer;
pub mod i18n_helpers;
pub mod identicon;
pub mod invite;
pub mod logging;
pub mod notifications;
pub mod persistence;
pub mod settings;
pub mod signaling;
pub mod state;
pub mod theater;
pub mod user_status;
pub mod utils;
pub mod webrtc;

// Include the auto-generated i18n module from build.rs (leptos_i18n_build).
// This creates `pub mod i18n { ... }` with Locale, I18nContextProvider, use_i18n, t!, etc.
include!(concat!(env!("OUT_DIR"), "/mod.rs"));

use leptos::prelude::*;
use logging::provide_logger_state;
use settings::provide_settings_state;
use state::provide_app_state;
use wasm_bindgen::prelude::wasm_bindgen;

/// Initialize and mount the Leptos application.
///
/// This function is automatically called by Trunk when the WASM module loads.
#[wasm_bindgen(start)]
pub fn init() {
  // Route Rust panics through the browser console so `RuntimeError:
  // unreachable` traces include the real panic message + Rust
  // file/line origin — without this hook, panics inside
  // `spawn_local` tasks are nearly impossible to diagnose from
  // the stripped WASM stack trace alone.
  console_error_panic_hook::set_once();

  // Mount the App component wrapped with I18nContextProvider.
  // Global states are provided inside mount_to_body so they live under
  // the correct reactive owner created by the mount call.
  mount_to_body(|| {
    let app_state = provide_app_state();
    provide_logger_state();
    let settings_state = provide_settings_state();

    // Initialize blacklist + invite manager early so signaling-message
    // handlers (registered while wiring the signaling client below) can
    // route inbound invitations through them immediately.
    let blacklist = blacklist::provide_blacklist_state();
    let invite_mgr = invite::provide_invite_manager();

    // Initialize config once and provide via context
    crate::config::provide_config();

    // Initialize error toast manager BEFORE signaling client so that
    // any ErrorResponse received during connection setup can be
    // displayed without a missing-context panic.
    let error_toast = error_handler::provide_error_toast_manager();

    // Wire the invite manager's cleanup tick to the toast surface so
    // client-side 60 s timeouts (Req 9.8) and "no one accepted"
    // multi-invite resolutions (Req 9.12) reach the user even when the
    // server never re-broadcasts the timeout.
    {
      let toast_for_obs = error_toast;
      invite_mgr.set_tick_observer(move |outcome| {
        for _ in &outcome.outbound_timed_out {
          toast_for_obs.show_info_message_with_key(
            "DSC902",
            "discovery.invite_expired",
            "An invitation timed out.",
          );
        }
        for (_bid, progress) in &outcome.batches_completed {
          if progress.is_unanswered() {
            toast_for_obs.show_info_message_with_key(
              "DSC903",
              "discovery.multi_invite_no_acceptance",
              "No one accepted the invitation; multi-user chat was not created.",
            );
          }
        }
      });
    }

    // Logout side-effect: cancel any pending blacklist auto-decline
    // timers and drop in-flight invites so they cannot fire after the
    // WebSocket has been torn down.
    {
      let blacklist = blacklist.clone();
      let invite_mgr_for_effect = invite_mgr.clone();
      Effect::new(move |_| {
        if app_state.auth.with(Option::is_none) {
          blacklist.cancel_all_auto_decline();
          invite_mgr_for_effect.clear_state();
        }
      });
    }
    // Suppress the unused-variable warning while keeping the binding
    // visible for future call sites that need a non-context handle.
    let _ = blacklist;
    let _ = invite_mgr;

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
    // console.warn.
    if let Some(logger) = leptos::prelude::use_context::<logging::LoggerState>() {
      user_status.set_logger(logger);
    }

    // Initialize WebRTC manager and provide via context
    let webrtc_manager = webrtc::provide_webrtc_manager(app_state);
    webrtc_manager.set_signaling_client(signaling.clone());

    // Initialize chat manager (Task 16) and cross-link with WebRTC so
    // outbound chat traffic reaches the DataChannel and inbound
    // DataChannel chat messages land in the reactive chat state.
    let chat_manager = chat::provide_chat_manager();
    chat_manager.set_webrtc(webrtc_manager.clone());
    webrtc_manager.set_chat_manager(chat_manager.clone());

    // Initialize persistence manager (Task 17) and attach to the chat
    // manager so messages are saved to / loaded from IndexedDB.
    let pm = persistence::provide_persistence_manager();
    chat_manager.set_persistence(pm.clone());

    // Reconcile pin / mute / archive flags against the IndexedDB
    // `conversation_flags` store now that the `PersistenceManager`
    // context has been provided (Req 7.7d). The reconcile runs
    // asynchronously inside `spawn_local`, so the UI shows the
    // localStorage-cached state on first paint and any IDB-only
    // updates land on the next reactive tick. Calling this from
    // `provide_app_state()` would no-op because that runs before
    // the PM context is installed.
    #[cfg(target_arch = "wasm32")]
    app_state.reconcile_conv_flags_from_idb();

    // Mirror the user-configured retention policy (Task 23 / Req 13.5)
    // into the persistence manager so the 72 h / 24 h / 7 d window
    // stays in sync with the UI selector.
    {
      let pm = pm.clone();
      let settings = settings_state;
      Effect::new(move |_| {
        let snapshot = settings.get();
        pm.set_retention_policy(snapshot.retention);
      });
    }

    // Initialize call manager (Task 18). Wire signaling + WebRTC so
    // CallInvite/Accept/Decline/End flow through the signaling client
    // and media-track add/replace flows through the WebRTC mesh.
    let call_manager = call::provide_call_manager(app_state);
    call_manager.set_signaling(signaling);
    call_manager.set_webrtc(webrtc_manager.clone());

    // Initialize file-transfer manager (Task 19) and link it to the
    // WebRTC mesh so outbound chunks reach their peers and inbound
    // FileMetadata / FileChunk frames find a registered handler.
    let file_manager = file_transfer::provide_file_transfer_manager();
    file_manager.set_webrtc(webrtc_manager.clone());
    webrtc_manager.set_file_transfer_manager(file_manager);

    // Initialize theater state (Task 22 — Req 12). Pure presentation
    // state owned by the Leptos component tree; the WebRTC / DataChannel
    // paths are wired up lazily by the theater components themselves.
    theater::provide_theater_state();

    // Run a maintenance tick (retention sweep + index rebuild) every
    // 60 seconds.
    {
      let cm = chat_manager.clone();
      let _maintenance = crate::utils::set_interval(60_000, move || {
        cm.run_maintenance();
      });
      // Leak the handle intentionally — the interval lives for the
      // entire application lifetime.
      std::mem::forget(_maintenance);
    }

    // Attempt to recover auth state from localStorage
    auth::try_recover_auth(app_state);

    view! {
      <i18n::I18nContextProvider>
        <app::App />
      </i18n::I18nContextProvider>
    }
  });
}
