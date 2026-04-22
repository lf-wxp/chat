//! Authentication service and context.
//!
//! Provides HTTP-based registration and login, JWT token persistence,
//! and automatic token recovery on page refresh.

mod auth_page;
mod login_form;
mod register_form;
mod token;

use leptos::prelude::*;
use message::UserId;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

pub(crate) use token::{KEY_USER_ID, KEY_USERNAME};
pub use token::{
  clear_auth_storage, load_active_call, load_active_room_id, load_auth_from_storage,
  load_avatar_from_storage, save_active_call, save_active_room_id, save_auth_to_storage,
};

pub use auth_page::AuthPage;

use crate::state::{AppState, AuthState};

/// Extract a user-friendly message from a `JsValue` error.
///
/// Attempts to read `.message` from the JS error object. Falls back to
/// a plain string if possible and finally to `JSON.stringify` so that
/// opaque JS objects still produce useful trace information instead of
/// a generic "Unknown error" (Issue-6 fix; R2-Issue-8 hardening).
fn format_js_error(e: &JsValue) -> String {
  // Explicit null / undefined branches so logs do not degrade to
  // "Unknown error" when a fetch() rejects with `undefined` (happens in
  // Firefox when the page is navigated away mid-request) (P2-4 fix).
  if e.is_null() {
    return "null".to_string();
  }
  if e.is_undefined() {
    return "undefined".to_string();
  }
  // Try DOMException first (fetch failures, abort errors, network errors, etc.)
  if let Some(dom_err) = e.dyn_ref::<web_sys::DomException>() {
    return dom_err.message();
  }
  // Try Error.prototype.message (TypeError, RangeError, etc.)
  if let Some(err) = e.dyn_ref::<js_sys::Error>() {
    return err.message().into();
  }
  // Fall back to stringifying the value directly.
  if let Some(s) = e.as_string() {
    return s;
  }
  // Last resort: run JSON.stringify so non-Error JS objects still reveal
  // their shape in logs instead of degrading to "Unknown error".
  if let Ok(json) = js_sys::JSON::stringify(e)
    && let Some(s) = json.as_string()
    && !s.is_empty()
    && s != "{}"
  {
    return s;
  }
  "Unknown error".to_string()
}

/// Decode the payload section of a JWT token.
///
/// Returns the decoded JSON payload string, or `None` if the token is
/// malformed or the decode fails. This is the WASM-only half of
/// [`is_jwt_expired`]; the pure-Rust expiry logic lives in
/// [`is_payload_expired`].
///
/// Uses the Rust `base64` crate with base64url (URL-safe, no padding)
/// instead of `window.atob()` so that non-ASCII string claims (e.g.
/// Unicode usernames) are decoded correctly as UTF-8 rather than being
/// corrupted by `atob()`'s Latin1 interpretation.
fn decode_jwt_payload(token: &str) -> Option<String> {
  let parts: Vec<&str> = token.split('.').collect();
  if parts.len() != 3 {
    return None;
  }
  let payload_b64 = parts[1];

  use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
  let bytes = URL_SAFE_NO_PAD.decode(payload_b64).ok()?;
  String::from_utf8(bytes).ok()
}

/// Check whether a decoded JWT payload has expired.
///
/// Pure Rust — does not depend on browser APIs and can be fully unit
/// tested on native targets (Issue-3 fix).
///
/// # Arguments
///
/// * `payload` — The decoded JSON payload string (e.g. from
///   [`decode_jwt_payload`]).
/// * `now_secs` — The current Unix timestamp in seconds.
///
/// # Logic
///
/// - Missing `exp` → treated as non-expiring (server has final say).
/// - Non-numeric `exp` → treated as expired (fail-safe).
/// - `nbf` claim is honoured with a [`JWT_CLOCK_SKEW_SECS`]-second grace
///   window to tolerate minor client/server clock drift (P1-3 fix).
pub(crate) fn is_payload_expired(payload: &str, now_secs: u64) -> bool {
  let parsed: serde_json::Value = match serde_json::from_str(payload) {
    Ok(v) => v,
    Err(_) => return true,
  };

  // Honour `nbf` (Not Before). A token whose `nbf` is in the future is
  // not yet valid; treat it as "expired" so the caller clears it. We
  // allow up to `JWT_CLOCK_SKEW_SECS` of client clock drift to avoid
  // rejecting tokens simply because the browser clock is a few seconds
  // ahead of the server (P1-3 fix).
  if let Some(nbf_val) = parsed.get("nbf")
    && let Some(nbf_secs) = nbf_val
      .as_u64()
      .or_else(|| nbf_val.as_f64().map(|f| f as u64))
    && nbf_secs > now_secs.saturating_add(JWT_CLOCK_SKEW_SECS)
  {
    return true;
  }

  // Only accept numeric `exp` claims. A string, boolean, or missing-but-
  // typed-wrong value is treated as expired so we never forward a token
  // the server would certainly reject (R2-Issue-9 hardening).
  match parsed.get("exp") {
    Some(v) => match v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)) {
      Some(exp_secs) => exp_secs <= now_secs,
      // Non-numeric `exp` — fail-safe: treat as expired.
      None => true,
    },
    // No exp claim — treat as non-expiring (server will validate).
    None => false,
  }
}

/// Check whether a JWT token has expired by decoding the payload `exp` claim.
///
/// Performs a lightweight base64 decode of the JWT payload without verifying
/// the signature. Returns `true` if the token is expired or cannot be parsed
/// (fail-safe: treat unparseable tokens as expired so the caller clears them).
///
/// # Server contract
///
/// This function assumes the server *always* issues tokens with a valid
/// numeric `exp` claim. If the backend ever starts issuing perpetual
/// tokens (no `exp`), the `None => false` branch below will treat such
/// tokens as non-expiring. Review this function together with
/// `server/src/auth/token.rs` whenever the token policy changes.
///
/// # WASM-only
///
/// This function depends on `web_sys::window().atob()` for base64 decoding
/// and will return `true` (treat as expired) on non-WASM targets where the
/// browser `window` object is unavailable. Do **not** call this from native
/// code paths — it is only meaningful inside a WASM browser context.
#[doc(hidden)]
pub fn is_jwt_expired(token: &str) -> bool {
  let Some(decoded) = decode_jwt_payload(token) else {
    return true;
  };
  let now_secs = (js_sys::Date::now() / 1000.0) as u64;
  is_payload_expired(&decoded, now_secs)
}

/// Permitted clock skew between the browser and the issuing server,
/// expressed in seconds. Used only for `nbf` validation; `exp` is checked
/// exactly so we never cling to an already-expired token.
pub(crate) const JWT_CLOCK_SKEW_SECS: u64 = 60;

/// Minimum password length for client-side validation.
///
/// Must stay in sync with the server-side policy in
/// `server/src/auth/validator.rs`. Changing this value requires updating
/// both sides and the i18n string `auth.password_too_short`.
pub(crate) const MIN_PASSWORD_LENGTH: usize = 8;

/// Registration request payload.
#[derive(Debug, Serialize)]
struct RegisterRequest {
  username: String,
  password: String,
}

/// Login request payload.
#[derive(Debug, Serialize)]
struct LoginRequest {
  username: String,
  password: String,
}

/// Auth API response.
#[derive(Debug, Deserialize)]
struct AuthResponse {
  user_id: String,
  token: String,
}

/// Auth API error response.
#[derive(Debug, Deserialize)]
struct AuthErrorResponse {
  error: String,
}

/// Result of an auth operation.
#[derive(Debug, Clone)]
pub struct AuthResult {
  /// Whether the operation succeeded.
  pub success: bool,
  /// Error message if the operation failed.
  pub error: Option<String>,
}

impl AuthResult {
  /// Create a successful result.
  pub fn ok() -> Self {
    Self {
      success: true,
      error: None,
    }
  }

  /// Create a failed result with an error message.
  pub fn err(msg: impl Into<String>) -> Self {
    Self {
      success: false,
      error: Some(msg.into()),
    }
  }
}

/// Get the base URL for auth API requests.
fn auth_base_url() -> String {
  crate::config::use_config().http_url
}

/// Register a new user.
///
/// Sends a POST request to `/api/register` with the given credentials.
/// On success, stores the JWT token and user info in `AppState` and
/// localStorage, then connects the signaling client.
pub fn register(
  username: String,
  password: String,
  app_state: AppState,
  on_result: impl Fn(AuthResult) + 'static,
) {
  let url = format!("{}/api/register", auth_base_url());
  let display_username = username.clone();
  send_auth_request::<RegisterRequest>(
    url,
    RegisterRequest { username, password },
    display_username,
    app_state,
    on_result,
  );
}

/// Log in an existing user.
///
/// Sends a POST request to `/api/login` with the given credentials.
/// On success, stores the JWT token and user info in `AppState` and
/// localStorage, then connects the signaling client.
pub fn login(
  username: String,
  password: String,
  app_state: AppState,
  on_result: impl Fn(AuthResult) + 'static,
) {
  let url = format!("{}/api/login", auth_base_url());
  let display_username = username.clone();
  send_auth_request::<LoginRequest>(
    url,
    LoginRequest { username, password },
    display_username,
    app_state,
    on_result,
  );
}

/// Attempt to recover auth state from localStorage and reconnect.
///
/// Called on app startup. If a valid token is found in localStorage,
/// performs a lightweight local JWT expiry check (Issue-5 fix) before
/// setting the auth state and connecting the signaling client (which
/// will send `TokenAuth` to verify the token with the server).
///
/// If `signaling.connect()` fails synchronously (e.g. malformed ws URL,
/// browser CSP block), the auth state is rolled back and storage is
/// cleared so the UI falls through to the login page instead of
/// appearing "logged in but disconnected" (P1-1 fix).
pub fn try_recover_auth(app_state: AppState) -> bool {
  if let Some(auth) = load_auth_from_storage() {
    // Quick local check: if the JWT has expired, skip the connect
    // attempt and go straight to the login page (Issue-5 fix).
    if is_jwt_expired(&auth.token) {
      clear_auth_storage();
      return false;
    }
    app_state.auth.set(Some(auth));
    let client = crate::signaling::use_signaling_client();
    if let Err(e) = client.connect() {
      // Roll back the optimistic auth update so the UI doesn't show a
      // "logged in" state that will never receive `AuthSuccess`
      // (P1-1 fix). The connect_with_url path already surfaced a toast
      // and stopped the reconnect loop.
      web_sys::console::error_1(&JsValue::from_str(&format!(
        "[auth] Recovery aborted — signaling connect failed: {}",
        e
      )));
      clear_auth_storage();
      app_state.auth.set(None);
      return false;
    }
    true
  } else {
    false
  }
}

/// Send an auth HTTP request.
///
/// The signaling client is captured *before* entering the async block so
/// that `expect_context` runs while the Leptos reactive owner is still
/// in scope. Inside `spawn_local` the owner may have been dropped, which
/// causes `use_signaling_client()` to panic (Bug-signaling-context).
fn send_auth_request<T: Serialize + 'static>(
  url: String,
  body: T,
  username: String,
  app_state: AppState,
  on_result: impl Fn(AuthResult) + 'static,
) {
  // Capture the signaling client reference while still inside the
  // reactive owner's scope (before entering the async closure).
  let signaling_client = crate::signaling::use_signaling_client();

  spawn_local(async move {
    match send_http_request(&url, &body).await {
      Ok(response) => {
        let user_id = match uuid::Uuid::parse_str(&response.user_id) {
          Ok(uuid) => UserId::from_uuid(uuid),
          Err(_) => {
            on_result(AuthResult::err("Invalid user ID from server"));
            return;
          }
        };

        let avatar = crate::identicon::generate_identicon_data_uri(&username);
        let auth = AuthState {
          user_id,
          token: response.token,
          nickname: username.clone(), // Use username as initial nickname until AuthSuccess
          username,
          avatar,
          signature: String::new(), // set later via settings UI
        };

        save_auth_to_storage(&auth);
        app_state.auth.set(Some(auth));

        // If the signaling connection cannot even be created (malformed
        // URL, blocked by CSP, ...), roll back the freshly-persisted auth
        // state so the UI doesn't get stuck on a "logged in but
        // disconnected" screen (P1-1 fix). The connect path has already
        // surfaced a user-visible toast.
        if let Err(e) = signaling_client.connect() {
          clear_auth_storage();
          app_state.auth.set(None);
          on_result(AuthResult::err(format!(
            "Signaling connection failed: {}",
            e
          )));
          return;
        }

        on_result(AuthResult::ok());
      }
      Err(e) => {
        on_result(AuthResult::err(e));
      }
    }
  });
}

/// Send an HTTP POST request and parse the auth response.
async fn send_http_request<T: Serialize>(url: &str, body: &T) -> Result<AuthResponse, String> {
  /// Request timeout in milliseconds (Optimisation 3).
  const REQUEST_TIMEOUT_MS: i32 = 10_000;

  let window = web_sys::window().ok_or("no window")?;

  let json_body = serde_json::to_string(body).map_err(|e| e.to_string())?;

  // Build an AbortController so we can cancel the request if it hangs.
  let abort_controller = web_sys::AbortController::new()
    .map_err(|e| format!("AbortController: {}", format_js_error(&e)))?;
  let abort_signal = abort_controller.signal();

  let request_init = web_sys::RequestInit::new();
  request_init.set_method("POST");
  request_init.set_signal(Some(&abort_signal));
  let headers = web_sys::Headers::new().map_err(|e| format_js_error(&e))?;
  headers
    .set("Content-Type", "application/json")
    .map_err(|e| format_js_error(&e))?;
  request_init.set_headers(&headers.into());
  request_init.set_body(&JsValue::from_str(&json_body));

  let request = web_sys::Request::new_with_str_and_init(url, &request_init)
    .map_err(|e| format!("Request creation failed: {}", format_js_error(&e)))?;

  // Schedule abort after the timeout.  We use `Closure::wrap` and keep
  // a handle so the closure is dropped deterministically after the
  // request completes instead of being leaked via `once_into_js` (Opt-C).
  let abort_controller_for_timeout = abort_controller.clone();
  let timeout_cb = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
    abort_controller_for_timeout.abort();
  }) as Box<dyn Fn()>);
  let timeout_id = window
    .set_timeout_with_callback_and_timeout_and_arguments_0(
      timeout_cb.as_ref().unchecked_ref(),
      REQUEST_TIMEOUT_MS,
    )
    .map_err(|e| format!("setTimeout failed: {}", format_js_error(&e)))?;

  let fetch_result = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
    .await
    .map_err(|e| {
      if abort_signal.aborted() {
        "Request timed out".to_string()
      } else {
        // P2-4 fix: Detect network/CORS errors and return a dedicated
        // i18n key so the UI can display a specific message instead of
        // the generic JS error string (which is often opaque like
        // "Failed to fetch" in Chrome or empty in Firefox).
        let msg = format_js_error(&e);
        if msg.is_empty()
          || msg.contains("Failed to fetch")
          || msg.contains("NetworkError")
          || msg.contains("Network request failed")
        {
          "auth.network_error".to_string()
        } else {
          format!("Fetch failed: {}", msg)
        }
      }
    });

  // Cancel the pending abort -- we finished in time.
  window.clear_timeout_with_handle(timeout_id);
  // Explicitly drop the closure so WASM heap memory is reclaimed (Opt-C).
  drop(timeout_cb);

  let response_value = fetch_result?;
  let response: web_sys::Response = response_value
    .dyn_into::<web_sys::Response>()
    .map_err(|_| "Fetch did not return a Response object".to_string())?;

  let status = response.status();
  let body_promise = response.text().map_err(|e| format_js_error(&e))?;
  let body_text = wasm_bindgen_futures::JsFuture::from(body_promise)
    .await
    .map_err(|e| format_js_error(&e))?
    .as_string()
    .unwrap_or_default();

  if (200..300).contains(&status) {
    parse_auth_success_response(&body_text)
  } else {
    Err(parse_auth_error_response(&body_text, status))
  }
}

/// Parse a successful (2xx) auth response body into an `AuthResponse`.
///
/// Extracted as a pure function for testability (T5 extraction).
fn parse_auth_success_response(body: &str) -> Result<AuthResponse, String> {
  serde_json::from_str::<AuthResponse>(body)
    .map_err(|e| format!("Failed to parse auth response: {} (body: {})", e, body))
}

/// Parse a non-2xx auth response body into an error message string.
///
/// Extracted as a pure function for testability (T5 extraction).
fn parse_auth_error_response(body: &str, status: u16) -> String {
  let error_response = serde_json::from_str::<AuthErrorResponse>(body);
  error_response
    .map(|r| r.error)
    .unwrap_or_else(|_| format!("HTTP {} error", status))
}

#[cfg(test)]
mod tests;
