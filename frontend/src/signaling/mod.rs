//! WebSocket signaling client.
//!
//! Manages the WebSocket connection lifecycle including connection,
//! reconnection with exponential backoff, heartbeat, and message
//! dispatch. Uses binary protocol (bitcode + frame encoding) from
//! the shared `message` crate.

mod connection;
mod message_handler;
mod reconnect;

use leptos::prelude::*;
use wasm_bindgen::JsValue;

use crate::state::AppState;

pub use connection::SignalingClient;

// ── Shared logging helpers (Opt-B) ──
//
// All signaling sub-modules route log messages through the structured
// `LoggerState` logger when available, falling back to raw
// `web_sys::console` during teardown or when the Leptos context is
// unavailable.

pub(crate) const LOG_MODULE: &str = "signaling";

pub(crate) fn log_debug(msg: &str) {
  if let Some(logger) = use_context::<crate::logging::LoggerState>() {
    logger.debug(LOG_MODULE, msg, None);
  } else {
    web_sys::console::log_1(&JsValue::from_str(msg));
  }
}

pub(crate) fn log_info(msg: &str) {
  if let Some(logger) = use_context::<crate::logging::LoggerState>() {
    logger.info(LOG_MODULE, msg, None);
  } else {
    web_sys::console::log_1(&JsValue::from_str(msg));
  }
}

pub(crate) fn log_warn(msg: &str) {
  if let Some(logger) = use_context::<crate::logging::LoggerState>() {
    logger.warn(LOG_MODULE, msg, None);
  } else {
    web_sys::console::warn_1(&JsValue::from_str(msg));
  }
}

pub(crate) fn log_error(msg: &str) {
  if let Some(logger) = use_context::<crate::logging::LoggerState>() {
    logger.error(LOG_MODULE, msg, None);
  } else {
    web_sys::console::error_1(&JsValue::from_str(msg));
  }
}

/// Initialize the signaling client and connect to the server.
///
/// This should be called once after the user is authenticated.
/// It creates a `SignalingClient` and provides it via Leptos context.
pub fn provide_signaling_client(
  app_state: AppState,
  user_status: crate::user_status::UserStatusManager,
  error_toast: crate::error_handler::ErrorToastManager,
) -> SignalingClient {
  let ws_url = crate::config::use_config().ws_url;
  let client = SignalingClient::new(ws_url, app_state, user_status, error_toast);
  provide_context(client.clone());
  client
}

/// Retrieve the signaling client from Leptos context.
///
/// # Panics
/// Panics if `provide_signaling_client` has not been called.
#[must_use]
pub fn use_signaling_client() -> SignalingClient {
  expect_context::<SignalingClient>()
}
