//! Settings drawer shell.
//!
//! Slides in from the right as an overlay, with a backdrop and entry /
//! exit animation. Provides access to every user-configurable
//! preference without replacing the main chat view.

use super::appearance_section::AppearanceSection;
use super::av_section::{AvSection, DeviceCache};
use super::data_management_section::DataManagementSection;
use super::notifications_section::NotificationsSection;
use super::privacy_section::PrivacySection;
use crate::components::discovery::BlacklistManagementPanel;
use crate::i18n;
use crate::settings::use_settings_state;
use crate::signaling::use_signaling_client;
use crate::state::use_app_state;
use icondata as i;
use leptos::ev::keydown;
use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use leptos_icons::Icon;
use leptos_use::{use_document, use_event_listener};

/// Context signal that indicates whether Do-Not-Disturb mode is
/// Settings drawer (slide-in panel).
#[component]
pub fn SettingsPage() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let app_state = use_app_state();
  let settings = use_settings_state();
  let open = app_state.settings_open;

  let close = move || open.set(false);

  // Provide a device-list cache that outlives the drawer toggle so
  // `AvSection` does not re-enumerate on every open (V3-M-3).
  provide_context(DeviceCache::new());

  // DND active signal — computed at this level so the top-of-drawer
  // Banner can read it (Req 13.4.5).
  let dnd_active = Memo::new(move |_| {
    let _ = app_state.now_tick.get() / 60;
    let snapshot = settings.get();
    let date = js_sys::Date::new_0();
    let minutes_of_day = date.get_hours() * 60 + date.get_minutes();
    snapshot.dnd.contains(minutes_of_day)
  });

  // Escape-to-close (Req 13.6.4). The listener is installed once and
  // gates on `open` so it does not interfere with other consumers
  // (modal, popover, image preview) when the settings drawer is
  // hidden.
  let _ = use_event_listener(use_document(), keydown, move |ev| {
    if !open.get_untracked() {
      return;
    }
    if ev.key() == "Escape" {
      ev.stop_propagation();
      open.set(false);
    }
  });

  // "Saved" feedback (Req 13.6.3). Bumps every time the user mutates
  // any setting; we mirror the tick into a transient "show" signal
  // so the indicator can be unmounted between bursts (which lets the
  // CSS keyframe animation replay on every save). Each new tick
  // cancels any pending "hide" timer so rapid successive saves keep
  // the indicator on screen until the last save finishes counting
  // down — avoiding a stale 1.5 s timer hiding the indicator
  // immediately after a new save arrives.
  //
  // `TimeoutHandle` is not `Clone`, so we store it inside a
  // `StoredValue<Option<_>>` and access it via `update_value` (which
  // does not require Clone). Dropping the old handle inside the
  // update closure implicitly cancels the timer via its `Drop` /
  // explicit `cancel()` call.
  let saved_tick = settings.saved_tick();
  let saved_visible = RwSignal::new(false);
  let pending_hide: StoredValue<Option<crate::utils::TimeoutHandle>> = StoredValue::new(None);
  Effect::new(move |_| {
    let tick = saved_tick.get();
    if tick == 0 {
      // Skip the initial state so the indicator does not flash on
      // first mount.
      return;
    }
    // Cancel the in-flight hide timer (if any) so this new save
    // resets the 1.5 s countdown cleanly.
    pending_hide.update_value(|slot| {
      if let Some(handle) = slot.take() {
        handle.cancel();
      }
    });
    saved_visible.set(true);
    let handle = crate::utils::set_timeout_once(1_500, move || {
      saved_visible.set(false);
      pending_hide.update_value(|slot| {
        *slot = None;
      });
    });
    pending_hide.update_value(|slot| {
      *slot = handle;
    });
  });

  view! {
    // Backdrop -- clicking it dismisses the drawer. Always rendered so
    // the CSS transition can run; visibility is driven by `.is-open`.
    <div
      class=move || {
        if open.get() {
          "drawer-backdrop is-open"
        } else {
          "drawer-backdrop"
        }
      }
      data-testid="settings-backdrop"
      aria-hidden=move || (!open.get()).to_string()
      on:click=move |_| close()
    ></div>

    // Drawer panel
    <aside
      class=move || {
        if open.get() {
          "drawer drawer-right is-open"
        } else {
          "drawer drawer-right"
        }
      }
      role="dialog"
      aria-modal="true"
      aria-labelledby="settings-drawer-title"
      aria-hidden=move || (!open.get()).to_string()
      data-testid="settings-page"
    >
      // Header
      <header class="drawer-header">
        <h1 id="settings-drawer-title" class="drawer-title">
          <Icon icon=i::LuSettings attr:class="drawer-title-icon" />
          {t!(i18n, settings.title)}
        </h1>

        // "Saved" pill -- conditionally mounted so the CSS entrance
        // animation replays on every save (Req 13.6.3).
        <Show when=move || saved_visible.get()>
          <div
            class="settings-saved-indicator"
            data-testid="settings-saved-indicator"
            aria-live="polite"
            data-active="true"
          >
            <Icon icon=i::LuCheck />
            <span>{t!(i18n, settings.saved_indicator)}</span>
          </div>
        </Show>

        <button
          class="btn-icon drawer-close"
          aria-label=move || t_string!(i18n, common.close)
          on:click=move |_| close()
        >
          <Icon icon=i::LuX />
        </button>
      </header>

      // Body -- scrollable content area. We unmount the section
      // components while the drawer is closed so their internal
      // Effects (device enumeration, DND 1-minute tick, storage
      // estimate poll) stop running instead of idling in the
      // background (S-9).
      <div class="drawer-body">
        <Show when=move || open.get()>
          // DND Banner at the top of the settings page (Req 13.4.5).
          // Displayed when the current time falls within the
          // Do-Not-Disturb window so the user is immediately aware.
          <Show when=move || dnd_active.get()>
            <div class="settings-dnd-banner" role="alert" data-testid="dnd-banner">
              <Icon icon=i::LuMoon attr:class="settings-dnd-banner-icon" />
              <span>{t!(i18n, settings.dnd_active)}</span>
            </div>
          </Show>

          <AppearanceSection />
          <AvSection />
          <PrivacySection />
          <NotificationsSection />
          <DataManagementSection />

          // Privacy -- blacklist management
          <section class="settings-section" aria-labelledby="blacklist-heading">
            <h2 id="blacklist-heading" class="settings-section-title">
              <Icon icon=i::LuShieldOff attr:class="settings-section-icon" />
              {t!(i18n, discovery.blacklist)}
            </h2>
            <BlacklistManagementPanel />
          </section>

          // Account section
          <section class="settings-section" aria-labelledby="account-heading">
            <h2 id="account-heading" class="settings-section-title">
              <Icon icon=i::LuUser attr:class="settings-section-icon" />
              {t!(i18n, settings.sections_account)}
            </h2>
            <button
              class="btn-danger settings-logout"
              data-testid="settings-logout"
              on:click=move |_| {
                let signaling = use_signaling_client();
                signaling.logout();
                open.set(false);
              }
            >
              <Icon icon=i::LuLogOut />
              <span>{t!(i18n, auth.logout)}</span>
            </button>
          </section>
        </Show>
      </div>
    </aside>
  }
}
