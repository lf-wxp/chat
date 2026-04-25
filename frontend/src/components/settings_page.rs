//! Settings drawer component.
//!
//! Slides in from the right as an overlay, with a backdrop and entry/exit
//! animation. Provides access to appearance, language and account actions
//! without replacing the main chat view.

use crate::i18n;
use crate::signaling::use_signaling_client;
use crate::state::use_app_state;
use icondata as i;
use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use leptos_icons::Icon;

/// Settings drawer (slide-in panel).
#[component]
pub fn SettingsPage() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let app_state = use_app_state();
  let theme = app_state.theme;
  let locale = app_state.locale;
  let open = app_state.settings_open;

  let close = move || open.set(false);

  view! {
    // Backdrop -- clicking it dismisses the drawer. Always rendered so the
    // CSS transition can run; visibility is driven by the `.is-open` class.
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
        <button
          class="btn-icon drawer-close"
          aria-label=move || t_string!(i18n, common.close)
          on:click=move |_| close()
        >
          <Icon icon=i::LuX />
        </button>
      </header>

      // Body -- scrollable content area
      <div class="drawer-body">
        // Appearance section
        <section class="settings-section" aria-labelledby="appearance-heading">
          <h2 id="appearance-heading" class="settings-section-title">
            <Icon icon=i::LuPalette attr:class="settings-section-icon" />
            {t!(i18n, settings.appearance)}
          </h2>

          // Theme selector -- segmented button group
          <div class="settings-row">
            <label class="settings-label">{t!(i18n, settings.theme)}</label>
            <div class="segmented" role="group">
              <button
                class=move || {
                  if theme.get() == "light" {
                    "segmented-item is-active"
                  } else {
                    "segmented-item"
                  }
                }
                on:click=move |_| theme.set("light".to_string())
                aria-pressed=move || (theme.get() == "light").to_string()
              >
                <Icon icon=i::LuSun />
                <span>{t!(i18n, settings.theme_light)}</span>
              </button>
              <button
                class=move || {
                  if theme.get() == "dark" {
                    "segmented-item is-active"
                  } else {
                    "segmented-item"
                  }
                }
                on:click=move |_| theme.set("dark".to_string())
                aria-pressed=move || (theme.get() == "dark").to_string()
              >
                <Icon icon=i::LuMoon />
                <span>{t!(i18n, settings.theme_dark)}</span>
              </button>
              <button
                class=move || {
                  if theme.get() == "system" {
                    "segmented-item is-active"
                  } else {
                    "segmented-item"
                  }
                }
                on:click=move |_| theme.set("system".to_string())
                aria-pressed=move || (theme.get() == "system").to_string()
              >
                <Icon icon=i::LuMonitor />
                <span>{t!(i18n, settings.theme_system)}</span>
              </button>
            </div>
          </div>

          // Language selector
          <div class="settings-row">
            <label class="settings-label">
              <Icon icon=i::LuGlobe attr:class="settings-label-icon" />
              {t!(i18n, settings.language)}
            </label>
            <div class="segmented" role="group">
              <button
                class=move || {
                  if locale.get() == "en" {
                    "segmented-item is-active"
                  } else {
                    "segmented-item"
                  }
                }
                on:click=move |_| locale.set("en".to_string())
                aria-pressed=move || (locale.get() == "en").to_string()
              >
                <span>"English"</span>
              </button>
              <button
                class=move || {
                  if locale.get() == "zh-CN" {
                    "segmented-item is-active"
                  } else {
                    "segmented-item"
                  }
                }
                on:click=move |_| locale.set("zh-CN".to_string())
                aria-pressed=move || (locale.get() == "zh-CN").to_string()
              >
                <span>"中文"</span>
              </button>
            </div>
          </div>
        </section>

        // Account section
        <section class="settings-section" aria-labelledby="account-heading">
          <h2 id="account-heading" class="settings-section-title">
            <Icon icon=i::LuUser attr:class="settings-section-icon" />
            {t!(i18n, auth.login)}
          </h2>
          <button
            class="btn-danger settings-logout"
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
      </div>
    </aside>
  }
}
