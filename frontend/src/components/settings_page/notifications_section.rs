//! Notifications section (message / call toggles + DND window).

use super::class_helpers::toggle_root_class;
use super::permission_badge::{PermissionBadge, PermissionState};
use crate::i18n;
use crate::settings::use_settings_state;
use icondata as i;
use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use leptos_icons::Icon;
use leptos_use::use_window;

/// Module-level flag that prevents re-registering the Permissions API
/// `onchange` listener every time the settings drawer re-opens.
/// Without this, each open/close cycle would leak a new JS closure
/// via `cb.forget()` (Bug-1 from code review).
static PERMISSION_LISTENER_SUBSCRIBED: std::sync::atomic::AtomicBool =
  std::sync::atomic::AtomicBool::new(false);

/// Notifications section.
#[component]
pub fn NotificationsSection() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let settings = use_settings_state();

  let message_on = Memo::new(move |_| settings.get().message_notifications);
  let call_on = Memo::new(move |_| settings.get().call_notifications);
  let dnd_enabled = Memo::new(move |_| settings.get().dnd.enabled);
  let dnd_start = Memo::new(move |_| settings.get().dnd.start_minutes);
  let dnd_end = Memo::new(move |_| settings.get().dnd.end_minutes);

  let toggle_message = move |_| {
    settings.update(|s| s.message_notifications = !s.message_notifications);
  };
  let toggle_call = move |_| {
    settings.update(|s| s.call_notifications = !s.call_notifications);
  };
  let toggle_dnd = move |_| {
    settings.update(|s| s.dnd.enabled = !s.dnd.enabled);
  };

  // Reactive handle on the current notification permission  // badge + "Request" button stay in sync with browser state after
  // the user grants or denies the prompt (Req 13.4.6 / 13.4.7).
  //
  // Kept up-to-date via two mechanisms (V2-S-5):
  //   1. The periodic `minute_tick` (already used for DND) re-reads
  //      `Notification.permission` each minute, which covers browsers
  //      that do not implement the Permissions API change event.
  //   2. When `navigator.permissions` is available, we subscribe to
  //      the `notifications` `PermissionStatus.onchange` event for
  //      an immediate update the moment the user flips the setting.
  let window = use_window();
  let permission_state: RwSignal<String> = RwSignal::new(permission_state_label(&window));

  // Use minute_tick from app_state for periodic permission re-read.
  let app_state = crate::state::use_app_state();
  let now_tick = app_state.now_tick;
  let minute_tick = Memo::new(move |_| now_tick.get() / 60);

  // Periodic re-read, gated on the minute tick.
  let window_for_tick = window.clone();
  Effect::new(move |_| {
    let _ = minute_tick.get();
    let latest = permission_state_label(&window_for_tick);
    if latest != permission_state.get_untracked() {
      permission_state.set(latest);
    }
  });

  // Immediate re-read via Permissions API change event. We use
  // `forget()` to keep the JS closure alive for the page lifetime.
  // A module-level AtomicBool ensures we only register the listener
  // once, even if the component re-mounts across drawer open/close
  // cycles (Bug-1 fix from code review).
  let window_for_subscribe = window.clone();
  Effect::new(move |_| {
    if !PERMISSION_LISTENER_SUBSCRIBED.swap(true, std::sync::atomic::Ordering::SeqCst) {
      subscribe_permission_change(permission_state, &window_for_subscribe);
    }
  });

  let window_for_request = StoredValue::new(window.clone());

  view! {
    <section class="settings-section" aria-labelledby="notifications-heading">
      <h2 id="notifications-heading" class="settings-section-title">
        <Icon icon=i::LuBell attr:class="settings-section-icon" />
        {t!(i18n, settings.notifications)}
      </h2>

      // Message notifications
      <div class="settings-row settings-toggle-row">
        <div class="settings-toggle-meta">
          <label class="settings-label">{t!(i18n, settings.message_notifications)}</label>
        </div>
        <button
          class=move || toggle_root_class(message_on.get())
          role="switch"
          aria-label=move || t_string!(i18n, settings.message_notifications)
          aria-checked=move || message_on.get().to_string()
          on:click=toggle_message
          data-testid="toggle-message-notifications"
        >
          <span class="settings-toggle-thumb"></span>
        </button>
      </div>

      // Call notifications
      <div class="settings-row settings-toggle-row">
        <div class="settings-toggle-meta">
          <label class="settings-label">{t!(i18n, settings.call_notifications)}</label>
        </div>
        <button
          class=move || toggle_root_class(call_on.get())
          role="switch"
          aria-label=move || t_string!(i18n, settings.call_notifications)
          aria-checked=move || call_on.get().to_string()
          on:click=toggle_call
          data-testid="toggle-call-notifications"
        >
          <span class="settings-toggle-thumb"></span>
        </button>
      </div>

      // Do not disturb
      <div class="settings-row settings-toggle-row">
        <div class="settings-toggle-meta">
          <label class="settings-label">{t!(i18n, settings.do_not_disturb)}</label>
          <p class="settings-hint">{t!(i18n, settings.do_not_disturb_hint)}</p>
        </div>
        <button
          class=move || toggle_root_class(dnd_enabled.get())
          role="switch"
          aria-label=move || t_string!(i18n, settings.do_not_disturb)
          aria-checked=move || dnd_enabled.get().to_string()
          on:click=toggle_dnd
          data-testid="toggle-dnd"
        >
          <span class="settings-toggle-thumb"></span>
        </button>
      </div>

      <Show when=move || dnd_enabled.get()>
        <div class="settings-row settings-dnd-window">
          <label class="settings-inline-label" for="settings-dnd-start">
            {t!(i18n, settings.dnd_start)}
          </label>
          <input
            id="settings-dnd-start"
            type="time"
            class="settings-time-input"
            prop:value=move || minutes_to_time_string(dnd_start.get())
            on:input=move |ev| {
              if let Some(minutes) = time_string_to_minutes(&event_target_value(&ev)) {
                settings.update(|s| s.dnd.start_minutes = minutes);
              }
            }
          />
          <label class="settings-inline-label" for="settings-dnd-end">
            {t!(i18n, settings.dnd_end)}
          </label>
          <input
            id="settings-dnd-end"
            type="time"
            class="settings-time-input"
            prop:value=move || minutes_to_time_string(dnd_end.get())
            on:input=move |ev| {
              if let Some(minutes) = time_string_to_minutes(&event_target_value(&ev)) {
                settings.update(|s| s.dnd.end_minutes = minutes);
              }
            }
          />
        </div>
      </Show>

      <p class="settings-hint">
        {t!(i18n, settings.dnd_permission_note)}
        " "
        <PermissionBadge state=PermissionState::from_browser_str(&permission_state.get()) />
      </p>

      // "Request Notification Permission" button (Req 13.4.6). Only
      // rendered when the browser exposes the API and the user has
      // not yet answered the prompt (`default`). Once the permission
      // changes to `granted` or `denied` the button disappears; the
      // badge above conveys the resulting state.
      <Show when=move || permission_state.get() == "default">
        <div class="settings-row">
          <button
            class="btn-primary settings-action"
            on:click=move |_| {
              let win_ref = window_for_request.get_value();
              wasm_bindgen_futures::spawn_local(async move {
                if let Ok(promise) = web_sys::Notification::request_permission()
                  && let Ok(_value) = wasm_bindgen_futures::JsFuture::from(promise).await
                {
                  permission_state.set(permission_state_label(&win_ref));
                }
              });
            }
            data-testid="request-notification-permission"
          >
            <Icon icon=i::LuBellRing />
            <span>{t!(i18n, settings.request_notification_permission)}</span>
          </button>
        </div>
      </Show>
    </section>
  }
}

/// Format a minutes-since-midnight value as `HH:MM`.
fn minutes_to_time_string(minutes: u32) -> String {
  let hours = minutes / 60;
  let mins = minutes % 60;
  format!("{hours:02}:{mins:02}")
}

/// Parse `HH:MM` back to minutes since midnight. Returns `None` when
/// the input is malformed.
fn time_string_to_minutes(value: &str) -> Option<u32> {
  let mut parts = value.split(':');
  let hours: u32 = parts.next()?.parse().ok()?;
  let minutes: u32 = parts.next()?.parse().ok()?;
  if hours >= 24 || minutes >= 60 {
    return None;
  }
  Some(hours * 60 + minutes)
}

/// Best-effort read of the current notification permission. Returns
/// a stable lowercase keyword the caller can map to an i18n label.
///
/// Accepts a `UseWindow` reference (from `leptos_use::use_window`) so
/// the caller does not need to reach into `web_sys::window()` directly.
fn permission_state_label(window: &leptos_use::UseWindow) -> String {
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
fn subscribe_permission_change(state: RwSignal<String>, window: &leptos_use::UseWindow) {
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
  fn minutes_to_string_pads_zeroes() {
    assert_eq!(minutes_to_time_string(0), "00:00");
    assert_eq!(minutes_to_time_string(9 * 60 + 5), "09:05");
    assert_eq!(minutes_to_time_string(23 * 60 + 59), "23:59");
  }

  #[test]
  fn string_to_minutes_round_trip() {
    for minutes in (0..24 * 60).step_by(37) {
      let text = minutes_to_time_string(minutes);
      assert_eq!(time_string_to_minutes(&text), Some(minutes));
    }
  }

  #[test]
  fn string_to_minutes_rejects_invalid() {
    assert_eq!(time_string_to_minutes(""), None);
    assert_eq!(time_string_to_minutes("25:00"), None);
    assert_eq!(time_string_to_minutes("12:99"), None);
    assert_eq!(time_string_to_minutes("abc"), None);
    assert_eq!(time_string_to_minutes("12"), None);
  }
}
