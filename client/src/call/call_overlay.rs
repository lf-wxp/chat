//! Call info overlay component
//!
//! Displays peer avatar, name, and current call status text.

use leptos::prelude::*;
use leptos_i18n::t_string;

use crate::{
  components::{Avatar, AvatarSize},
  i18n::*,
};

use super::types::{CallStatus, format_duration};

/// Call info overlay showing peer avatar, name, and status
#[component]
pub fn CallOverlay(
  /// Peer display name
  #[prop(into)]
  peer_name: String,
  /// Current call status
  call_status: RwSignal<CallStatus>,
  /// Call duration in seconds (only meaningful when InCall)
  call_duration: RwSignal<u32>,
) -> impl IntoView {
  let i18n = use_i18n();

  let format_duration_display = move || {
    let secs = call_duration.get();
    format_duration(secs)
  };

  view! {
    <div class="call-overlay">
      <div class="call-peer-info">
        <Avatar username=peer_name.clone() size=AvatarSize::Large />
        <div class="call-peer-name">{peer_name}</div>
        <div class="call-status-text">
          {move || match call_status.get() {
            CallStatus::Idle => t_string!(i18n, call_ready).to_string(),
            CallStatus::Calling => t_string!(i18n, call_calling_status).to_string(),
            CallStatus::Ringing => t_string!(i18n, call_ringing_status).to_string(),
            CallStatus::InCall => format_duration_display(),
          }}
        </div>
      </div>
    </div>
  }
}
