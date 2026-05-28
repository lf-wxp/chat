//! Incoming-call modal.
//!
//! Rendered whenever the local `CallState` transitions to
//! [`CallState::Ringing`]. Offers Accept / Decline buttons and
//! displays the caller's nickname, avatar, and media type.
//!
//! Layout / animation / dismissal are delegated to the shared
//! `ModalWrapper`. Backdrop-click is **disabled** so a stray click
//! cannot silently decline the call — the user must explicitly choose
//! Accept or Decline. Escape is also disabled for the same reason
//! (matches platform conventions for incoming-call sheets).

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_i18n::t_string;
use message::types::MediaType;
use message::{UserId, types::UserInfo};

use crate::call::{CallState, use_call_manager, use_call_signals};
use crate::components::room::modal_wrapper::{ModalSize, ModalWrapper};
use crate::i18n;
use crate::state::use_app_state;

/// Whether the given [`CallState`] should cause the incoming-call
/// modal to be rendered. Extracted as a pure helper so the decision
/// is testable without mounting the component.
#[must_use]
pub const fn should_render_modal(state: &CallState) -> bool {
  matches!(state, CallState::Ringing { .. })
}

/// Resolve the caller's display name from the online-users directory.
///
/// Falls back to the stringified user id when the caller is not
/// currently tracked as online (e.g. a cross-room invite landed before
/// the presence broadcast).
#[must_use]
pub fn resolve_caller_name(online_users: &[UserInfo], caller: &UserId) -> String {
  online_users
    .iter()
    .find(|u| u.user_id == *caller)
    .map(|u| u.nickname.clone())
    .unwrap_or_else(|| caller.to_string())
}

/// Incoming-call modal component.
#[component]
pub fn IncomingCallModal() -> impl IntoView {
  let signals = use_call_signals();
  let manager = use_call_manager();
  let app_state = use_app_state();
  let i18n = i18n::use_i18n();

  let is_ringing = Memo::new(move |_| should_render_modal(&signals.call_state.get()));
  let is_open = Signal::derive(move || is_ringing.get());

  // Extract caller / media info reactively so the view does not need
  // to destructure the enum inline.
  let caller_name = Memo::new(move |_| match signals.call_state.get() {
    CallState::Ringing { from, .. } => app_state
      .online_users
      .with(|users| resolve_caller_name(users, &from)),
    _ => String::new(),
  });

  let media_label = Memo::new(move |_| match signals.call_state.get() {
    CallState::Ringing { media_type, .. } => match media_type {
      MediaType::Video => t_string!(i18n, call.video_call),
      MediaType::Audio | MediaType::ScreenShare => t_string!(i18n, call.call),
    },
    _ => "",
  });

  // Clone the manager twice so the accept + decline click handlers each
  // have their own owned copy.
  let manager_accept = manager.clone();
  let manager_decline = manager;

  // No-op on_close: the modal is dismissed by the call-state machine
  // (Accept / Decline button click), not by the wrapper. We still pass
  // a callback because ModalWrapper requires one.
  let on_close = Callback::new(|()| {});

  view! {
    <ModalWrapper
      on_close=on_close
      open=is_open
      size=ModalSize::Small
      class="call-modal"
      labelled_by="call-modal-title"
      testid="incoming-call-modal"
      dismiss_on_backdrop_click=false
      dismiss_on_escape=false
    >
      <header class="call-modal__header">
        <h2 id="call-modal-title" class="call-modal__title">
          {move || t_string!(i18n, call.incoming_call)}
        </h2>
      </header>
      <div class="call-modal__body">
        <p class="call-modal__caller">{move || caller_name.get()}</p>
        <p class="call-modal__meta">{move || media_label.get()}</p>
      </div>
      <footer class="call-modal__actions">
        <button
          type="button"
          class="btn btn--danger"
          on:click={
            let manager = manager_decline.clone();
            move |_| manager.decline_call()
          }
          aria-label=move || t_string!(i18n, call.decline)
          data-testid="call-decline-btn"
        >
          {move || t_string!(i18n, call.decline)}
        </button>
        <button
          type="button"
          class="btn btn--primary"
          on:click={
            let manager = manager_accept.clone();
            move |_| {
              let manager = manager.clone();
              spawn_local(async move {
                if let Err(e) = manager.accept_call().await {
                  web_sys::console::error_1(
                    &format!("[call] accept failed: {e}").into(),
                  );
                }
              });
            }
          }
          aria-label=move || t_string!(i18n, call.accept)
          data-testid="call-accept-btn"
        >
          {move || t_string!(i18n, call.accept)}
        </button>
      </footer>
    </ModalWrapper>
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::call::CallEndReason;
  use message::types::UserStatus;
  use message::{RoomId, UserId};

  fn room() -> RoomId {
    RoomId::from_uuid(uuid::Uuid::new_v5(
      &uuid::Uuid::NAMESPACE_DNS,
      b"modal-test",
    ))
  }

  fn user(seed: &str) -> UserId {
    UserId::from_uuid(uuid::Uuid::new_v5(
      &uuid::Uuid::NAMESPACE_DNS,
      seed.as_bytes(),
    ))
  }

  #[test]
  fn ringing_state_renders_modal() {
    let state = CallState::Ringing {
      room_id: room(),
      media_type: MediaType::Audio,
      from: user("alice"),
      received_at_ms: 0,
    };
    assert!(should_render_modal(&state));
  }

  #[test]
  fn idle_state_hides_modal() {
    assert!(!should_render_modal(&CallState::Idle));
  }

  #[test]
  fn active_state_hides_modal() {
    let state = CallState::Active {
      room_id: room(),
      media_type: MediaType::Video,
      started_at_ms: 0,
    };
    assert!(!should_render_modal(&state));
  }

  #[test]
  fn ended_state_hides_modal() {
    let state = CallState::Ended {
      reason: CallEndReason::RemoteEnded,
    };
    assert!(!should_render_modal(&state));
  }

  #[test]
  fn caller_name_found_in_directory() {
    let alice = user("alice");
    let users = vec![UserInfo {
      user_id: alice.clone(),
      username: "alice".to_string(),
      nickname: "Alice".to_string(),
      status: UserStatus::Online,
      avatar_url: None,
      bio: String::new(),
      created_at_nanos: 0,
      last_seen_nanos: 0,
    }];
    assert_eq!(resolve_caller_name(&users, &alice), "Alice");
  }

  #[test]
  fn caller_name_falls_back_to_user_id() {
    let alice = user("alice");
    let name = resolve_caller_name(&[], &alice);
    assert_eq!(name, alice.to_string());
  }
}
