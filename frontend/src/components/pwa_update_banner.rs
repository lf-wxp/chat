//! PWA update banner component.
//!
//! Bridges the service-worker update lifecycle (registered in
//! `index.html`) with the Leptos UI. When the browser installs a new
//! version of the service worker in the background, `sw.js` posts a
//! notification through `window.__pwaUpdateAvailable`. This component
//! exposes that hook via a reactive signal and renders a banner that
//! lets the user opt in to refresh on their own schedule.
//!
//! The update flow:
//!   1. Browser fetches and installs the new `sw.js`.
//!   2. The `statechange` handler in `index.html` invokes
//!      `window.__pwaUpdateAvailable()`.
//!   3. This component flips its `available` signal to `true`.
//!   4. The user clicks "Update" → we post `SKIP_WAITING` to the
//!      waiting worker and reload the page so the new worker takes
//!      control.

use crate::i18n;
use icondata as i;
use leptos::prelude::*;
use leptos_i18n::t_string;
use leptos_icons::Icon;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;
use web_sys::js_sys;

/// PWA update banner.
///
/// Visible only after the service worker reports that a newer version
/// has finished installing and is waiting to activate. The banner is
/// non-blocking: users can continue using the app and update whenever
/// convenient. A successful update triggers a full reload so the new
/// WASM bundle is loaded from the new cache generation.
#[component]
pub fn PwaUpdateBanner() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let available = RwSignal::new(false);
  let dismissed = RwSignal::new(false);

  // Wire `window.__pwaUpdateAvailable` once per mount. The closure is
  // stored on the window object so `sw.js`' `statechange` listener can
  // call it; we deliberately `forget()` it because the window outlives
  // the component for the lifetime of the page.
  Effect::new(move |prev: Option<()>| {
    if prev.is_some() {
      // Only install the bridge once.
      return;
    }
    let Some(window) = web_sys::window() else {
      return;
    };
    let cb = Closure::wrap(Box::new(move || {
      available.set(true);
    }) as Box<dyn Fn()>);
    if let Err(e) = js_sys::Reflect::set(
      &window,
      &JsValue::from_str("__pwaUpdateAvailable"),
      cb.as_ref().unchecked_ref(),
    ) {
      web_sys::console::warn_1(&e);
    }
    cb.forget();
  });

  let show = move || available.get() && !dismissed.get();

  let on_update = move |_| {
    // Ask the waiting worker to skip waiting so the new version
    // activates immediately, then reload once it takes control.
    if let Some(window) = web_sys::window() {
      let navigator = window.navigator();
      // Navigator.serviceWorker is not exposed in web_sys' typed bindings
      // uniformly across versions; reach it via Reflect to stay resilient.
      let sw_container = js_sys::Reflect::get(&navigator, &JsValue::from_str("serviceWorker")).ok();
      if let Some(container) = sw_container {
        let ready_promise = js_sys::Reflect::get(&container, &JsValue::from_str("ready")).ok();
        if let Some(ready) = ready_promise {
          let ready_promise: js_sys::Promise = ready.unchecked_into();
          // `Promise::then` expects `FnMut` (it may retry internally),
          // so declare the closure as `FnMut` even though we only fire
          // once in practice.
          let reload_closure = Closure::wrap(Box::new(move |reg: JsValue| {
            if let Ok(waiting) = js_sys::Reflect::get(&reg, &JsValue::from_str("waiting"))
              && !waiting.is_null()
              && !waiting.is_undefined()
            {
              let msg = js_sys::Object::new();
              let _ = js_sys::Reflect::set(
                &msg,
                &JsValue::from_str("type"),
                &JsValue::from_str("SKIP_WAITING"),
              );
              if let Ok(post_fn) = js_sys::Reflect::get(&waiting, &JsValue::from_str("postMessage"))
              {
                let post_fn: js_sys::Function = post_fn.unchecked_into();
                let _ = post_fn.call1(&waiting, &msg);
              }
            }
            // Reload to pick up the fresh assets.
            if let Some(w) = web_sys::window()
              && let Ok(loc) = js_sys::Reflect::get(&w, &JsValue::from_str("location"))
              && let Ok(reload_fn) = js_sys::Reflect::get(&loc, &JsValue::from_str("reload"))
            {
              let reload_fn: js_sys::Function = reload_fn.unchecked_into();
              let _ = reload_fn.call0(&loc);
            }
          }) as Box<dyn FnMut(JsValue)>);
          let _ = ready_promise.then(&reload_closure);
          reload_closure.forget();
          return;
        }
      }
      // No ServiceWorker available — just reload as a best-effort
      // fallback so the user still ends up on the new bundle.
      if let Ok(loc) = js_sys::Reflect::get(&window, &JsValue::from_str("location"))
        && let Ok(reload_fn) = js_sys::Reflect::get(&loc, &JsValue::from_str("reload"))
      {
        let reload_fn: js_sys::Function = reload_fn.unchecked_into();
        let _ = reload_fn.call0(&loc);
      }
    }
  };

  let on_dismiss = move |_| {
    dismissed.set(true);
  };

  view! {
    <Show when=show>
      <div
        class="pwa-update-banner"
        role="status"
        aria-live="polite"
      >
        <Icon icon=i::LuRefreshCw attr:class="pwa-update-banner__icon" />
        <span class="pwa-update-banner__text">
          {move || t_string!(i18n, pwa.update_available)}
        </span>
        <button
          type="button"
          class="pwa-update-banner__btn pwa-update-banner__btn--primary"
          on:click=on_update
        >
          {move || t_string!(i18n, pwa.update_button)}
        </button>
        <button
          type="button"
          class="pwa-update-banner__btn"
          aria-label=move || t_string!(i18n, error.dismiss_banner)
          on:click=on_dismiss
        >
          {move || t_string!(i18n, error.dismiss_banner)}
        </button>
      </div>
    </Show>
  }
}
