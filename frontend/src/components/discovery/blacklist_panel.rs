//! Blacklist management panel (Req 9.20).
//!
//! Designed to be embedded inside the Settings drawer. Lists every
//! blocked user with their display name, block timestamp and an
//! Unblock action. Empty state communicates that the blacklist is
//! local-only and never synchronised with the server (Req 9.21).
//!
//! When the same user is also currently online, the row prefers the
//! latest nickname / username from `app_state.online_users` over the
//! snapshot stored at block time so a renamed user appears with their
//! current display name (Opt-3 fix).

use leptos::prelude::*;
use leptos_i18n::{t, t_string};

use crate::blacklist::{BlacklistEntry, use_blacklist_state};
use crate::i18n;
use crate::identicon::generate_identicon_data_uri;
use crate::state::use_app_state;

#[component]
pub fn BlacklistManagementPanel() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let blacklist = use_blacklist_state();
  let app_state = use_app_state();

  // Derive a sorted list that subscribes to the same underlying signal
  // as `blacklist.list()`. `Signal::derive` is the idiomatic Leptos 0.8
  // equivalent of the hand-rolled `Memo::new(|_| { let _ = sig.get(); list })`
  // pattern — it drops one layer of reactive caching (Opt-4.2 fix).
  let entries = {
    let bl = blacklist.clone();
    Signal::derive(move || bl.list())
  };

  view! {
    <section
      class="blacklist-panel"
      aria-labelledby="blacklist-panel-title"
      data-testid="blacklist-panel"
    >
      <header class="blacklist-panel__header">
        <h3 id="blacklist-panel-title" class="blacklist-panel__title">
          {t!(i18n, discovery.blacklist)}
        </h3>
        <p class="blacklist-panel__subtitle">{t!(i18n, discovery.blacklist_local_note)}</p>
      </header>

      <Show
        when=move || !entries.get().is_empty()
        fallback=move || view! {
          <p class="blacklist-panel__empty">{t!(i18n, discovery.blacklist_empty)}</p>
        }
      >
        <ul class="blacklist-panel__list" role="list">
          <For
            each=move || entries.get()
            key=|entry: &BlacklistEntry| entry.user_id.clone()
            children={
              let bl = blacklist.clone();
              move |entry: BlacklistEntry| {
                let user_id = entry.user_id.clone();
                let blocked_at = entry.blocked_at_ms;
                let unblock_label = t_string!(i18n, discovery.unblock);
                let bl_for_click = bl.clone();
                let user_id_for_click = user_id.clone();

                // Resolve the freshest display name reactively so a
                // user who renamed themselves after being blocked
                // shows up with their current nickname / username
                // (Opt-3 fix). Falls back to the snapshot stored on
                // the entry when the user is offline.
                let entry_for_name = entry.clone();
                let online_users = app_state.online_users;
                let display_name = Memo::new(move |_| {
                  online_users.with(|list| {
                    list
                      .iter()
                      .find(|u| u.user_id == entry_for_name.user_id)
                      .map(|u| {
                        if u.nickname.is_empty() {
                          u.username.clone()
                        } else {
                          u.nickname.clone()
                        }
                      })
                      .unwrap_or_else(|| entry_for_name.display_name.clone())
                  })
                });
                let avatar = Memo::new(move |_| generate_identicon_data_uri(&display_name.get()));

                view! {
                  <li class="blacklist-panel__row" data-testid="blacklist-row">
                    <img
                      class="blacklist-panel__avatar"
                      src=move || avatar.get()
                      alt=""
                      width="32"
                      height="32"
                    />
                    <div class="blacklist-panel__info">
                      <span class="blacklist-panel__name">{move || display_name.get()}</span>
                      <span class="blacklist-panel__time">
                        {format_blocked_at(blocked_at)}
                      </span>
                    </div>
                    <button
                      type="button"
                      class="btn btn--ghost blacklist-panel__unblock"
                      aria-label=move || format!("Unblock {}", display_name.get())
                      on:click=move |_| bl_for_click.unblock(&user_id_for_click)
                    >
                      {unblock_label}
                    </button>
                  </li>
                }
              }
            }
          />
        </ul>
      </Show>
    </section>
  }
}

/// Format a unix-ms timestamp as a UTC "yyyy-MM-dd HH:mm UTC" string.
/// Falls back to the raw timestamp string if `chrono` cannot interpret
/// the value (which only happens for nonsense input).
///
/// Uses `chrono::Utc` instead of `chrono::Local` so the output is
/// deterministic across CI environments (Opt-8 / Opt-11 fix).
fn format_blocked_at(ms: i64) -> String {
  use chrono::TimeZone;
  chrono::Utc
    .timestamp_millis_opt(ms)
    .single()
    .map(|dt| format!("{} UTC", dt.format("%Y-%m-%d %H:%M")))
    .unwrap_or_else(|| ms.to_string())
}

#[cfg(test)]
mod tests {
  use super::format_blocked_at;

  #[test]
  fn format_blocked_at_returns_non_empty_string() {
    let formatted = format_blocked_at(1_700_000_000_000);
    // UTC output is deterministic: "2023-11-14 22:13 UTC" (16+ chars).
    assert!(formatted.len() >= 14, "got {formatted}");
    assert!(
      formatted.ends_with("UTC"),
      "should end with UTC, got {formatted}"
    );
  }

  #[test]
  fn format_blocked_at_handles_zero() {
    let formatted = format_blocked_at(0);
    assert!(!formatted.is_empty());
    assert!(
      formatted.ends_with("UTC"),
      "should end with UTC, got {formatted}"
    );
  }
}
