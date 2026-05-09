//! PWA "Add to Home Screen" install prompt component.
//!
//! Captures the browser's `beforeinstallprompt` event (Chrome / Edge /
//! Android) and defers it so we can surface a custom, branded install
//! banner instead of the browser's default mini-infobar. On iOS Safari
//! the event never fires — the banner stays hidden and users rely on
//! the Share → "Add to Home Screen" flow (this is browser-imposed).
//!
//! Once the user dismisses the prompt we record a timestamp in
//! localStorage so we don't re-pester them on every page load. The
//! cool-down defaults to 14 days.

use crate::i18n;
use crate::utils;
use icondata as i;
use leptos::prelude::*;
use leptos_i18n::t_string;
use leptos_icons::Icon;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;
use web_sys::js_sys;

/// localStorage key used to suppress the prompt after dismissal.
const DISMISSED_AT_KEY: &str = "pwa_install_dismissed_at_ms";

/// Cool-down after a dismissal before we may show the prompt again.
/// 14 days strikes a balance between respecting the user and making
/// sure they can find the feature again later.
const DISMISS_COOLDOWN_MS: f64 = 14.0 * 24.0 * 60.0 * 60.0 * 1000.0;

/// Wrapper around the captured `beforeinstallprompt` event so it can
/// live inside a Leptos [`StoredValue`].
///
/// `JsValue` is `!Send + !Sync`, which Leptos' typed `StoredValue` does
/// not require — but the broader reactive machinery does. The
/// project's single-threaded WASM invariant lets us safely opt in via
/// [`wasm_send_sync!`].
#[derive(Default)]
struct DeferredPrompt(Option<JsValue>);

crate::wasm_send_sync!(DeferredPrompt);

/// PWA install prompt banner.
///
/// Hidden until the browser fires `beforeinstallprompt`. Once visible
/// the user can either accept (which triggers the native prompt UI)
/// or dismiss (which starts a 14-day cool-down). Auto-hides itself
/// when the browser reports `appinstalled`.
#[component]
pub fn PwaInstallPrompt() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let can_show = RwSignal::new(false);
  // Stored as `StoredValue` so the captured event survives across
  // callback boundaries without tripping Leptos' `Send + Sync` bound.
  let deferred = StoredValue::new(DeferredPrompt::default());

  // One-shot effect: install the `beforeinstallprompt` and
  // `appinstalled` listeners exactly once per mount.
  Effect::new(move |prev: Option<()>| {
    if prev.is_some() {
      return;
    }
    let Some(window) = web_sys::window() else {
      return;
    };

    // Honour any prior dismissal before we even attach the listener.
    if recently_dismissed() {
      return;
    }

    // beforeinstallprompt: capture and suppress the default mini-infobar.
    let bip_cb = Closure::wrap(Box::new(move |event: JsValue| {
      // Calling preventDefault lets us own the UI.
      if let Ok(prevent_default) =
        js_sys::Reflect::get(&event, &JsValue::from_str("preventDefault"))
      {
        let prevent_default: js_sys::Function = prevent_default.unchecked_into();
        let _ = prevent_default.call0(&event);
      }
      deferred.update_value(|slot| slot.0 = Some(event));
      can_show.set(true);
    }) as Box<dyn Fn(JsValue)>);
    let _ = window
      .add_event_listener_with_callback("beforeinstallprompt", bip_cb.as_ref().unchecked_ref());
    bip_cb.forget();

    // appinstalled: clean up the prompt and mark the user as installed.
    let installed_cb = Closure::wrap(Box::new(move || {
      deferred.update_value(|slot| slot.0 = None);
      can_show.set(false);
      utils::save_to_local_storage("pwa_installed", "1");
    }) as Box<dyn Fn()>);
    let _ = window
      .add_event_listener_with_callback("appinstalled", installed_cb.as_ref().unchecked_ref());
    installed_cb.forget();
  });

  let on_install = move |_| {
    let event = deferred.try_update_value(|slot| slot.0.take()).flatten();
    let Some(event) = event else {
      can_show.set(false);
      return;
    };
    // Call event.prompt() to show the native install dialog.
    if let Ok(prompt_fn) = js_sys::Reflect::get(&event, &JsValue::from_str("prompt")) {
      let prompt_fn: js_sys::Function = prompt_fn.unchecked_into();
      let _ = prompt_fn.call0(&event);
    }
    // Hide our banner immediately; `appinstalled` / `userChoice`
    // fires asynchronously and either resolution removes the event
    // permanently anyway.
    can_show.set(false);
  };

  let on_dismiss = move |_| {
    can_show.set(false);
    let now_ms = js_sys::Date::now();
    utils::save_to_local_storage(DISMISSED_AT_KEY, &now_ms.to_string());
  };

  let show = move || can_show.get();

  view! {
    <Show when=show>
      <div
        class="pwa-install-prompt"
        role="dialog"
        aria-live="polite"
      >
        <Icon icon=i::LuDownload attr:class="pwa-install-prompt__icon" />
        <div class="pwa-install-prompt__body">
          <span class="pwa-install-prompt__title">
            {move || t_string!(i18n, pwa.install_title)}
          </span>
          <span class="pwa-install-prompt__message">
            {move || t_string!(i18n, pwa.install_message)}
          </span>
        </div>
        <div class="pwa-install-prompt__actions">
          <button
            type="button"
            class="pwa-install-prompt__btn pwa-install-prompt__btn--primary"
            on:click=on_install
          >
            {move || t_string!(i18n, pwa.install_button)}
          </button>
          <button
            type="button"
            class="pwa-install-prompt__btn"
            on:click=on_dismiss
          >
            {move || t_string!(i18n, pwa.dismiss_install)}
          </button>
        </div>
      </div>
    </Show>
  }
}

/// Returns `true` when the user dismissed the prompt within the
/// cool-down window. When storage is unavailable or the value cannot
/// be parsed, falls back to *not* suppressing the prompt so new users
/// still see it.
fn recently_dismissed() -> bool {
  let Some(raw) = utils::load_from_local_storage(DISMISSED_AT_KEY) else {
    return false;
  };
  let Ok(ts_ms) = raw.parse::<f64>() else {
    return false;
  };
  let now_ms = js_sys::Date::now();
  (now_ms - ts_ms) < DISMISS_COOLDOWN_MS
}
