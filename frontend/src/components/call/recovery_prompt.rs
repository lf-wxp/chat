//! Refresh-recovery confirmation prompt.
//!
//! When a persisted call state is found in localStorage on bootstrap,
//! this modal asks the user whether they want to rejoin the previous
//! call or discard it (Req 10.5).

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_i18n::t_string;

use crate::call::{PersistedCallState, use_call_manager, use_call_signals};
use crate::i18n;

/// Whether the prompt modal should render given the current
/// `recovery_prompt` signal value. Exposed as a pure helper so the
/// decision is unit-testable without mounting the component.
#[must_use]
pub const fn should_render_prompt(pending: Option<&PersistedCallState>) -> bool {
  pending.is_some()
}

/// Refresh-recovery prompt component.
#[component]
pub fn CallRecoveryPrompt() -> impl IntoView {
  let signals = use_call_signals();
  let manager = use_call_manager();
  let i18n = i18n::use_i18n();

  let pending = Memo::new(move |_| {
    signals
      .recovery_prompt
      .with(|p| should_render_prompt(p.as_ref()))
  });

  // The inner view is rebuilt whenever `pending` flips to `true`, so we
  // must clone `manager` into each click handler independently. Stored
  // closures cannot be used here because `Show`'s `ChildrenFn` requires
  // `Fn` (callable multiple times) and `on:click` takes the handler by
  // value.
  let manager_rejoin = manager.clone();
  let manager_discard = manager;

  view! {
    <Show when=move || pending.get()>
      <div
        class="call-modal-overlay"
        role="dialog"
        aria-modal="true"
        aria-labelledby="call-recovery-title"
      >
        <div class="call-modal">
          <header class="call-modal__header">
            <h2 id="call-recovery-title" class="call-modal__title">
              {move || t_string!(i18n, call.recovery_prompt)}
            </h2>
          </header>
          <div class="call-modal__body">
            <p>{move || t_string!(i18n, call.recovery_body)}</p>
          </div>
          <footer class="call-modal__actions">
            <button
              type="button"
              class="btn"
              on:click={
                let manager = manager_discard.clone();
                move |_| {
                  let manager = manager.clone();
                  spawn_local(async move {
                    manager.resolve_recovery(false).await;
                  });
                }
              }
            >
              {move || t_string!(i18n, call.recovery_discard)}
            </button>
            <button
              type="button"
              class="btn btn--primary"
              on:click={
                let manager = manager_rejoin.clone();
                move |_| {
                  let manager = manager.clone();
                  spawn_local(async move {
                    manager.resolve_recovery(true).await;
                  });
                }
              }
            >
              {move || t_string!(i18n, call.recovery_rejoin)}
            </button>
          </footer>
        </div>
      </div>
    </Show>
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::call::CallPhase;
  use message::RoomId;
  use message::types::MediaType;

  fn sample_state() -> PersistedCallState {
    PersistedCallState {
      room_id: RoomId::from_uuid(uuid::Uuid::new_v5(
        &uuid::Uuid::NAMESPACE_DNS,
        b"recovery-test",
      )),
      media_type: MediaType::Audio,
      started_at_ms: 42,
      screen_sharing: false,
      phase: CallPhase::Active,
    }
  }

  #[test]
  fn prompt_renders_when_pending_some() {
    let state = sample_state();
    assert!(should_render_prompt(Some(&state)));
  }

  #[test]
  fn prompt_hidden_when_pending_none() {
    assert!(!should_render_prompt(None));
  }
}
