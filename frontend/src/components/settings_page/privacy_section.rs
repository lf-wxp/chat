//! Privacy toggles (online-status visibility, read receipts).
//!
//! The blacklist management panel is rendered by the parent shell via
//! the existing `BlacklistManagementPanel` component.

use super::class_helpers::toggle_root_class;
use crate::i18n;
use crate::settings::use_settings_state;
use crate::user_status::use_user_status_manager;
use icondata as i;
use leptos::prelude::*;
use leptos_i18n::{t, t_string};
use leptos_icons::Icon;

/// Privacy section.
#[component]
pub fn PrivacySection() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let settings = use_settings_state();
  let user_status = use_user_status_manager();

  let online_visible = Memo::new(move |_| settings.get().online_status_visible);
  let read_receipts = Memo::new(move |_| settings.get().read_receipts);

  let toggle_online = {
    let user_status = user_status.clone();
    move |_| {
      settings.update(|s| s.online_status_visible = !s.online_status_visible);
      // Req 13.3.4 — flipping this toggle "SHALL immediately update
      // the user's online status broadcast behavior". Re-broadcast
      // the current status so peers observe the new visibility
      // state without waiting for the next status change.
      user_status.refresh_broadcast();
    }
  };
  let toggle_receipts = move |_| {
    settings.update(|s| s.read_receipts = !s.read_receipts);
  };

  view! {
    <section class="settings-section" aria-labelledby="privacy-heading">
      <h2 id="privacy-heading" class="settings-section-title">
        <Icon icon=i::LuShield attr:class="settings-section-icon" />
        {t!(i18n, settings.privacy)}
      </h2>

      // Online status visibility
      <div class="settings-row settings-toggle-row">
        <div class="settings-toggle-meta">
          <label class="settings-label">{t!(i18n, settings.online_status_visible)}</label>
          <p class="settings-hint">{t!(i18n, settings.online_status_hint)}</p>
        </div>
        <button
          class=move || toggle_root_class(online_visible.get())
          role="switch"
          aria-label=move || t_string!(i18n, settings.online_status_visible)
          aria-checked=move || online_visible.get().to_string()
          on:click=toggle_online
          data-testid="toggle-online-visible"
        >
          <span class="settings-toggle-thumb"></span>
        </button>
      </div>

      // Read receipts
      <div class="settings-row settings-toggle-row">
        <div class="settings-toggle-meta">
          <label class="settings-label">{t!(i18n, settings.read_receipts)}</label>
          <p class="settings-hint">{t!(i18n, settings.read_receipts_hint)}</p>
        </div>
        <button
          class=move || toggle_root_class(read_receipts.get())
          role="switch"
          aria-label=move || t_string!(i18n, settings.read_receipts)
          aria-checked=move || read_receipts.get().to_string()
          on:click=toggle_receipts
          data-testid="toggle-read-receipts"
        >
          <span class="settings-toggle-thumb"></span>
        </button>
      </div>
    </section>
  }
}
