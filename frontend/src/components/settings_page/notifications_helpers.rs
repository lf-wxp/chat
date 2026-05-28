//! Notification permission helpers.
//!
//! Extracted from `notifications_section.rs` for testability and
//! separation of concerns (O-5). Contains the permission-state
//! query and the Permissions API change-subscription logic.

use leptos::prelude::*;

/// Best-effort read of the current notification permission./// a stable lowercase keyword the caller can map to an i18n label.
///
/// Accepts a `UseWindow` reference (from `leptos_use::use_window`) so
/// the caller does not need to reach into `web_sys::window()` directly.
pub(super) fn permission_state_label(window: &leptos_use::UseWindow) -> String {
  let Some(window) = window.as_ref() else {
    return "unsupported".to_string();
  };
  let has_api = js_sys::Reflect::get(window, &wasm_bindgen::JsValue::from_str("Notification"))
    .map(|v| !v.is_undefined() && !v.is_null())
    .unwrap_or(false);
  if !has_api {
    return "unsupported".to_string();
  }
  let permission = web_sys::Notification::permission();
  match permission {
    web_sys::NotificationPermission::Granted => "granted".to_string(),
    web_sys::NotificationPermission::Denied => "denied".to_string(),
    _ => "default".to_string(),
  }
}

/// Subscribe to the browser's Permissions API so the UI reflects
/// permission changes made outside the page (e.g. the user toggling
/// the site setting in the browser UI). On runtimes that do not
/// expose `navigator.permissions` (older Safari, some WebViews) this
/// is a silent no-op and the `minute_tick` Effect in the caller
/// still keeps the badge in sync within a minute (V2-S-5).
///
/// The JS closure is leaked via `forget()` — this is intentional in
/// WASM single-threaded environments where the listener must live
/// for the entire page session. The caller guards against duplicate
/// subscriptions with a `subscribed` flag.
///
/// `state` is an `ArcRwSignal` so the leaked closure can continue to
/// update it after every settings-drawer mount / unmount cycle
/// without risking a "value already disposed" panic from the owner
pub(super) fn subscribe_permission_change(
  state: ArcRwSignal<String>,
  window: &leptos_use::UseWindow,
) {
  use wasm_bindgen::closure::Closure;
  use wasm_bindgen::{JsCast, JsValue};

  let Some(window) = window.as_ref() else {
    return;
  };
  let Ok(permissions_val) =
    js_sys::Reflect::get(&window.navigator(), &JsValue::from_str("permissions"))
  else {
    return;
  };
  if permissions_val.is_undefined() || permissions_val.is_null() {
    return;
  }
  let query_fn_val = match js_sys::Reflect::get(&permissions_val, &JsValue::from_str("query")) {
    Ok(v) => v,
    Err(_) => return,
  };
  let Ok(query_fn) = query_fn_val.dyn_into::<js_sys::Function>() else {
    return;
  };
  let descriptor = js_sys::Object::new();
  let _ = js_sys::Reflect::set(
    &descriptor,
    &JsValue::from_str("name"),
    &JsValue::from_str("notifications"),
  );
  let promise_val = match query_fn.call1(&permissions_val, &descriptor) {
    Ok(v) => v,
    Err(_) => return,
  };
  let Ok(promise) = promise_val.dyn_into::<js_sys::Promise>() else {
    return;
  };

  wasm_bindgen_futures::spawn_local(async move {
    let Ok(status) = wasm_bindgen_futures::JsFuture::from(promise).await else {
      return;
    };
    let cb = Closure::wrap(Box::new(move || {
      // Read permission directly — this closure outlives any component
      // scope so we cannot hold a UseWindow reference here.
      let latest = {
        let permission = web_sys::Notification::permission();
        match permission {
          web_sys::NotificationPermission::Granted => "granted".to_string(),
          web_sys::NotificationPermission::Denied => "denied".to_string(),
          _ => "default".to_string(),
        }
      };
      if latest != state.get_untracked() {
        state.set(latest);
      }
    }) as Box<dyn Fn()>);
    let _ = js_sys::Reflect::set(
      &status,
      &JsValue::from_str("onchange"),
      cb.as_ref().unchecked_ref(),
    );
    // Leak the closure so the JS callback stays alive for the page
    // session. In WASM single-threaded environments this is the
    // standard pattern for long-lived event listeners.
    cb.forget();
  });
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn permission_state_label_compiles() {
    // Verify the function exists and compiles. We cannot call
    // `permission_state_label` on non-WASM targets because it
    // depends on browser APIs (js_sys, web_sys). Full coverage
    // is provided by the WASM test suite.
    let _ = permission_state_label;
  }
}
