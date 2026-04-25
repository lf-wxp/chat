//! Auth HTTP service.
//!
//! Provides HTTP-based registration, login, and token recovery logic.

use leptos::prelude::*;
use message::UserId;
use serde::Serialize;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

use crate::state::{AppState, AuthState};

use super::jwt::is_jwt_expired;
use super::token::{clear_auth_storage, load_auth_from_storage, save_auth_to_storage};
use super::types::{AuthErrorResponse, AuthResponse, AuthResult, LoginRequest, RegisterRequest};
use super::utils::format_js_error;

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
      web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!(
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
  request_init.set_body(&wasm_bindgen::JsValue::from_str(&json_body));

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
