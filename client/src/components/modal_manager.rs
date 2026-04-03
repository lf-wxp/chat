//! Global modal manager
//!
//! Renders the corresponding modal component based on `UiState.active_modal`.

use leptos::prelude::*;

use crate::state;

use super::modals::{IncomingCallModal, InviteReceivedModal, UserProfileModal};

/// Global modal manager — mounted in App root component
#[component]
pub fn ModalManager() -> impl IntoView {
  let ui_state = state::use_ui_state();

  view! {
    {move || {
      let modal = ui_state.get().active_modal.clone();
      match modal {
        Some(state::ModalType::InviteReceived { from_user_id, from_username, message }) => {
          view! {
            <InviteReceivedModal
              from_user_id=from_user_id
              from_username=from_username
              message=message
            />
          }.into_any()
        }
        Some(state::ModalType::IncomingCall { from_user_id, from_username, is_video }) => {
          view! {
            <IncomingCallModal
              from_user_id=from_user_id
              from_username=from_username
              is_video=is_video
            />
          }.into_any()
        }
        Some(state::ModalType::UserProfile(user_id)) => {
          view! {
            <UserProfileModal user_id=user_id />
          }.into_any()
        }
        _ => {
          view! { <div></div> }.into_any()
        }
      }
    }}
  }
}
