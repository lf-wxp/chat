//! Notifications section (message / call toggles + DND window).

use super::class_helpers::toggle_root_class;
use super::notifications_helpers::{permission_state_label, subscribe_permission_change};
use super::permission_badge::{PermissionBadge, PermissionState};
use crate::i18n;
use crate::settings::use_settings_state;
use icondata as i;
use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use leptos_icons::Icon;
use leptos_use::use_window;
use std::sync::Mutex;

/// Module-level global permission state signal backed by an
/// `ArcRwSignal`. Unlike `RwSignal`, `ArcRwSignal` is NOT registered
/// with the reactive owner arena and therefore survives every
/// mount / unmount of `NotificationsSection` for the lifetime of the
/// page. This matters because:
///
/// 1. The Permissions API `onchange` closure (leaked via `forget()`)
///    captures this signal on first subscription and must remain
///    able to write to it after the settings drawer has been closed.
/// 2. Re-opening the drawer creates a *new* reactive owner scope, so
///    any `RwSignal` created in the previous scope has already been
///    disposed — accessing it would panic with
///    "you tried to access a reactive value ... that has already
///    been disposed".
///
/// `Mutex` is used only as a `Sync`-safe initialisation guard (we
/// are single-threaded on WASM so contention is a non-issue).
static PERMISSION_SIGNAL: Mutex<Option<ArcRwSignal<String>>> = Mutex::new(None);

/// Module-level flag that prevents re-registering the Permissions API
/// `onchange` listener every time the settings drawer re-opens.
static PERMISSION_LISTENER_SUBSCRIBED: std::sync::atomic::AtomicBool =
  std::sync::atomic::AtomicBool::new(false);

/// Initialise (or retrieve) the global permission-state `ArcRwSignal`.
/// Returns a clone — `ArcRwSignal` is reference-counted so all clones
/// observe the same underlying value.
fn global_permission_signal(window: &leptos_use::UseWindow) -> ArcRwSignal<String> {
  let mut guard = PERMISSION_SIGNAL
    .lock()
    .expect("permission signal mutex poisoned");
  if guard.is_none() {
    *guard = Some(ArcRwSignal::new(permission_state_label(window)));
  }
  guard
    .as_ref()
    .expect("permission signal initialised above")
    .clone()
}

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

  // DND time-input validation state (B-9). When the user types an
  // invalid time string the signal flips to `true` and a hint is
  // shown beneath the inputs.
  let dnd_start_invalid: RwSignal<bool> = RwSignal::new(false);
  let dnd_end_invalid: RwSignal<bool> = RwSignal::new(false);

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
  let permission_state = global_permission_signal(&window);

  // Use minute_tick from app_state for periodic permission re-read.
  let app_state = crate::state::use_app_state();
  let now_tick = app_state.now_tick;
  let minute_tick = Memo::new(move |_| now_tick.get() / 60);

  // Periodic re-read, gated on the minute tick.
  let window_for_tick = window.clone();
  let ps_for_tick = permission_state.clone();
  Effect::new(move |_| {
    let _ = minute_tick.get();
    let latest = permission_state_label(&window_for_tick);
    if latest != ps_for_tick.get_untracked() {
      ps_for_tick.set(latest);
    }
  });

  // Immediate re-read via Permissions API change event. We use
  // `forget()` to keep the JS closure alive for the page lifetime.
  // A module-level AtomicBool ensures we only register the listener
  // once, even if the component re-mounts across drawer open/close
  // cycles.
  let window_for_subscribe = window.clone();
  let ps_for_subscribe = permission_state.clone();
  Effect::new(move |_| {
    if !PERMISSION_LISTENER_SUBSCRIBED.swap(true, std::sync::atomic::Ordering::SeqCst) {
      subscribe_permission_change(ps_for_subscribe.clone(), &window_for_subscribe);
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
            class:settings-time-input-invalid=move || dnd_start_invalid.get()
            prop:value=move || minutes_to_time_string(dnd_start.get())
            on:input=move |ev| {
              let value = event_target_value(&ev);
              if let Some(minutes) = time_string_to_minutes(&value) {
                dnd_start_invalid.set(false);
                settings.update(|s| s.dnd.start_minutes = minutes);
              } else if !value.is_empty() {
                dnd_start_invalid.set(true);
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
            class:settings-time-input-invalid=move || dnd_end_invalid.get()
            prop:value=move || minutes_to_time_string(dnd_end.get())
            on:input=move |ev| {
              let value = event_target_value(&ev);
              if let Some(minutes) = time_string_to_minutes(&value) {
                dnd_end_invalid.set(false);
                settings.update(|s| s.dnd.end_minutes = minutes);
              } else if !value.is_empty() {
                dnd_end_invalid.set(true);
              }
            }
          />
        </div>
        <Show when=move || dnd_start_invalid.get() || dnd_end_invalid.get()>
          <p class="settings-error settings-dnd-validation">
            {t!(i18n, settings.dnd_time_invalid)}
          </p>
        </Show>
      </Show>

      <p class="settings-hint">
        {t!(i18n, settings.dnd_permission_note)}
        " "
        <PermissionBadge state=Signal::derive({
          let ps = permission_state.clone();
          move || PermissionState::from_browser_str(&ps.get())
        }) />
      </p>

      // "Request Notification Permission" button (Req 13.4.6). Only
      // rendered when the browser exposes the API and the user has
      // not yet answered the prompt (`default`). Once the permission
      // changes to `granted` or `denied` the button disappears; the
      // badge above conveys the resulting state.
      <Show when={
        let ps = permission_state.clone();
        move || ps.get() == "default"
      }>
        <div class="settings-row">
          <button
            class="btn-primary settings-action"
            on:click={
              let ps = permission_state.clone();
              move |_| {
                let win_ref = window_for_request.get_value();
                let ps_for_task = ps.clone();
                wasm_bindgen_futures::spawn_local(async move {
                  if let Ok(promise) = web_sys::Notification::request_permission()
                    && let Ok(_value) = wasm_bindgen_futures::JsFuture::from(promise).await
                  {
                    ps_for_task.set(permission_state_label(&win_ref));
                  }
                });
              }
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
