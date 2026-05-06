//! Owner disconnect grace-window banner (Req 12.2 §6a).
//!
//! Rendered at the top of the video surface while the owner is
//! reconnecting. A 1-second interval decrements the visible
//! countdown; once the grace window elapses the banner switches from
//! "reconnecting" messaging to "offline" messaging so the viewer can
//! choose to wait or leave (Req 12.2 §6a trailing sentence).
//!
//! The timer itself is driven by the theater state signals
//! (`owner_reconnecting` + `owner_grace_seconds`) so the banner only
//! owns presentation and does not attempt to synchronise wall clocks
//! with other components.

use js_sys::Date;
use leptos::prelude::*;
use leptos_i18n::{t, t_string};

use crate::i18n;
use crate::theater::{GRACE_WINDOW_SECONDS, compute_grace_remaining, use_theater_state};

/// Interval at which the grace-window countdown refreshes (ms).
const TICK_INTERVAL_MS: u32 = 1_000;

/// Banner component.
#[component]
pub fn TheaterGraceBanner() -> impl IntoView {
  let state = use_theater_state();
  let i18n = i18n::use_i18n();

  // Wall-clock anchor captured once per reconnect event. We store it
  // inside the state's `owner_grace_seconds` signal via a side effect
  // below so the banner and any concurrent observers agree on the
  // remaining seconds without drifting.
  let started_at_ms = RwSignal::<u64>::new(0);

  // Re-arm the anchor whenever the owner transitions into the
  // "reconnecting" state. Using `get_untracked` for the previous
  // reading avoids a spurious re-run cycle.
  Effect::new(move |_| {
    if state.owner_reconnecting.get() {
      started_at_ms.set(Date::now() as u64);
      state
        .owner_grace_seconds
        .set(u8::try_from(GRACE_WINDOW_SECONDS).unwrap_or(u8::MAX));
    }
  });

  // 1 Hz tick that refreshes the countdown until the window elapses.
  let tick = set_interval_with_handle(
    move || {
      if !state.owner_reconnecting.get_untracked() {
        return;
      }
      let started = started_at_ms.get_untracked();
      if started == 0 {
        return;
      }
      let remaining = compute_grace_remaining(started, Date::now() as u64, GRACE_WINDOW_SECONDS);
      state
        .owner_grace_seconds
        .set(u8::try_from(remaining).unwrap_or(u8::MAX));
    },
    std::time::Duration::from_millis(u64::from(TICK_INTERVAL_MS)),
  );
  if let Ok(handle) = tick {
    on_cleanup(move || handle.clear());
  }

  let message = move || {
    if state.owner_grace_seconds.get() == 0 {
      t_string!(i18n, theater.owner_offline).to_string()
    } else {
      t_string!(i18n, theater.owner_reconnecting)
        .to_string()
        .replace("{seconds}", &state.owner_grace_seconds.get().to_string())
    }
  };

  let banner_class = move || {
    if state.owner_grace_seconds.get() == 0 {
      "theater-grace-banner theater-grace-banner--offline"
    } else {
      "theater-grace-banner theater-grace-banner--reconnecting"
    }
  };

  view! {
    <Show when=move || state.owner_reconnecting.get()>
      <aside
        class=banner_class
        role="status"
        aria-live="assertive"
        data-testid="theater-grace-banner"
      >
        <span class="theater-grace-banner__message">{message}</span>
        <Show when=move || state.owner_grace_seconds.get() == 0>
          <button
            type="button"
            class="btn btn--ghost theater-grace-banner__leave"
            on:click=move |_| state.leave()
            data-testid="theater-grace-leave"
          >
            {t!(i18n, theater.leave)}
          </button>
        </Show>
      </aside>
    </Show>
  }
}
