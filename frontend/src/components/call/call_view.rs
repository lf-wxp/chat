//! Active-call view: video grid + control bar.
//!
//! Rendered whenever the call state is `Active` or `Inviting` (so the
//! caller sees their own preview while waiting for the callee to pick
//! up). The `Idle` / `Ringing` / `Ended` states are handled by sibling
//! components under [`crate::components::call`].

use leptos::prelude::*;
use leptos_i18n::t_string;

use crate::call::{CallState, use_call_signals};
use crate::components::call::{CallControls, VideoGrid};
use crate::i18n;

/// Active-call view component.
#[component]
pub fn CallView() -> impl IntoView {
  let signals = use_call_signals();
  let i18n = i18n::use_i18n();

  let should_render = Memo::new(move |_| {
    matches!(
      signals.call_state.get(),
      CallState::Active { .. } | CallState::Inviting { .. },
    )
  });

  let status_label = Memo::new(move |_| match signals.call_state.get() {
    CallState::Inviting { .. } => t_string!(i18n, call.inviting),
    CallState::Active { .. } => t_string!(i18n, call.active),
    _ => "",
  });

  view! {
    <Show when=move || should_render.get()>
      <section class="call-view" role="region" aria-label=move || t_string!(i18n, call.call)>
        <header class="call-view__header">
          <span class="call-view__status">{move || status_label.get()}</span>
        </header>
        <VideoGrid />
        <CallControls />
      </section>
    </Show>
  }
}
