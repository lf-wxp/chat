//! Modal manager component.
//!
//! Hosts global modals that can be triggered from anywhere in the app
//! (incoming room invites, future profile cards, …). Components inside
//! this manager render lazily based on `app_state` signals.

use leptos::prelude::*;

use crate::components::room::IncomingRoomInviteModal;

/// Modal manager component.
#[component]
pub fn ModalManager() -> impl IntoView {
  view! {
    <div id="modal-root">
      <IncomingRoomInviteModal />
    </div>
  }
}
