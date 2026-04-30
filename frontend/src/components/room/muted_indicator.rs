//! Muted-state banner shown above the chat input when the current
//! user has been silenced by an admin (Req 15.3 §21).
//!
//! Renders an accessible live-updating countdown for timed mutes and
//! a static "Permanently muted" label for indefinite ones. Designed
//! to be slotted above the message composer; it is entirely
//! presentational.

use icondata as i;
use leptos::prelude::*;
use leptos_i18n::t_string;
use leptos_icons::Icon;
use message::types::MemberInfo;

use crate::components::room::utils::{is_currently_muted, mute_remaining_seconds};
use crate::i18n;
use crate::state::use_app_state;

/// Muted input indicator.
#[component]
pub fn MutedIndicator(
  /// Reactive member info for the current user inside the active room.
  #[prop(into)]
  member: Signal<Option<MemberInfo>>,
) -> impl IntoView {
  let i18n = i18n::use_i18n();
  let app_state = use_app_state();
  // Subscribe to the global 1 Hz tick instead of running our own
  // interval (Sprint 4.3 of the review-task-21 follow-up).
  let tick = app_state.now_tick;

  let is_muted = Memo::new(move |_| {
    let _ = tick.get();
    member.with(|m| m.as_ref().is_some_and(is_currently_muted))
  });

  let countdown_text = Memo::new(move |_| {
    let _ = tick.get();
    member.with(|m| m.as_ref().and_then(mute_remaining_seconds))
  });

  let label_text = Memo::new(move |_| match countdown_text.get() {
    Some(secs) => {
      let minutes = secs / 60;
      let seconds = secs % 60;
      format!(
        "{} ({:02}:{:02})",
        t_string!(i18n, room.muted_indicator),
        minutes,
        seconds
      )
    }
    None => t_string!(i18n, room.muted_permanent).to_string(),
  });

  view! {
    <Show when=move || is_muted.get()>
      <div
        class="room-muted-indicator"
        role="status"
        aria-live="polite"
        data-testid="room-muted-indicator"
      >
        <span class="room-muted-indicator__icon" aria-hidden="true"><Icon icon=i::LuMicOff /></span>
        <span class="room-muted-indicator__text">{move || label_text.get()}</span>
      </div>
    </Show>
  }
}
