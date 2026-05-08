//! Settings drawer shell.
//!
//! Slides in from the right as an overlay, with a backdrop and entry /
//! exit animation. Provides access to every user-configurable
//! preference without replacing the main chat view.

use super::appearance_section::AppearanceSection;
use super::av_helpers::DeviceCache;
use super::av_section::AvSection;
use super::background_section::BackgroundSection;
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
use wasm_bindgen::JsCast;

/// Context signal that indicates whether Do-Not-Disturb mode is
/// Settings drawer (slide-in panel).
#[component]
pub fn SettingsPage() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let app_state = use_app_state();
  let settings = use_settings_state();
  let signaling = StoredValue::new(use_signaling_client());
  let open = app_state.settings_open;

  let close = move || open.set(false);

  // Provide a device-list cache that outlives the drawer toggle so
  // `AvSection` does not re-enumerate on every open (V3-M-3).
  provide_context(DeviceCache::new());

  // DND active signal — computed at this level so the top-of-drawer
  // Banner can read it (Req 13.4.5). Only re-evaluates when the
  // minute changes (not every second) to avoid creating a new
  // `js_sys::Date` object on every tick.
  //
  // Note: `app_state.now_tick` is a monotonically-increasing counter
  // (not a Unix timestamp), so we cannot derive the current
  // wall-clock time from it. We must use `js_sys::Date::new_0()`
  // inside the minute-granularity Memo instead, which limits the Date
  // allocation to once per minute.
  //
  // A previous review suggestion (A13) proposed deriving dnd_active
  // from now_tick directly, but this is infeasible: now_tick is a
  // free-running counter whose value bears no correlation to the
  // actual wall-clock hour/minute. The minute_tick Memo (derived
  // from now_tick / 60) is still useful as a reactivity trigger —
  // it ensures this Memo only reruns when the minute counter changes,
  // not on every 1-second tick — but the actual time-of-day check
  // requires a real clock source.
  let minute_tick = Memo::new(move |_| app_state.now_tick.get() / 60);
  let dnd_active = Memo::new(move |_| {
    let _ = minute_tick.get();
    let snapshot = settings.get();
    let date = js_sys::Date::new_0();
    let minutes_of_day = date
      .get_hours()
      .saturating_mul(60)
      .saturating_add(date.get_minutes());
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
      on:click=move |ev| {
        // Do not close the drawer if the click originated from a
        // modal-backdrop (e.g. the ConfirmDialog's overlay). The
        // modal handles its own dismissal; closing the drawer here
        // would dismiss both layers at once (B-3).
        let target = ev.target();
        if let Some(target) = target
          && let Some(el) = target.dyn_ref::<web_sys::Element>()
          && el.class_list().contains("modal-backdrop")
        {
          return;
        }
        close()
      }
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

          // Quick-nav anchors for fast section access (O-8).
          //
          // Evenly-distributed icon rail that stays sticky while the
          // settings body is scrolled. Each anchor jumps to the
          // matching `<section>` heading via its fragment id. The
          // row inherits its background / text colours from design
          // tokens so it follows the active theme automatically
          // (light / dark) instead of rendering as a stark white
          // block in dark mode.
          <nav
            class="settings-quick-nav"
            aria-label=move || t_string!(i18n, settings.title)
          >
            <a
              class="settings-quick-nav-link"
              href="#appearance-heading"
              aria-label=move || t_string!(i18n, settings.appearance)
              title=move || t_string!(i18n, settings.appearance)
            >
              <Icon icon=i::LuPalette />
            </a>
            <a
              class="settings-quick-nav-link"
              href="#av-heading"
              aria-label=move || t_string!(i18n, settings.sections_av)
              title=move || t_string!(i18n, settings.sections_av)
            >
              <Icon icon=i::LuVideo />
            </a>
            <a
              class="settings-quick-nav-link"
              href="#privacy-heading"
              aria-label=move || t_string!(i18n, settings.privacy)
              title=move || t_string!(i18n, settings.privacy)
            >
              <Icon icon=i::LuShield />
            </a>
            <a
              class="settings-quick-nav-link"
              href="#notifications-heading"
              aria-label=move || t_string!(i18n, settings.notifications)
              title=move || t_string!(i18n, settings.notifications)
            >
              <Icon icon=i::LuBell />
            </a>
            <a
              class="settings-quick-nav-link"
              href="#data-heading"
              aria-label=move || t_string!(i18n, settings.data_management)
              title=move || t_string!(i18n, settings.data_management)
            >
              <Icon icon=i::LuDatabase />
            </a>
            <a
              class="settings-quick-nav-link"
              href="#blacklist-heading"
              aria-label=move || t_string!(i18n, discovery.blacklist)
              title=move || t_string!(i18n, discovery.blacklist)
            >
              <Icon icon=i::LuShieldOff />
            </a>
            <a
              class="settings-quick-nav-link"
              href="#account-heading"
              aria-label=move || t_string!(i18n, settings.sections_account)
              title=move || t_string!(i18n, settings.sections_account)
            >
              <Icon icon=i::LuUser />
            </a>
          </nav>

          <AppearanceSection />
          <BackgroundSection />
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
                signaling.with_value(|s| s.logout());
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
