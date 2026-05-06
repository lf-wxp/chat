//! Shared permission badge component.
//!
//! Renders a small inline badge indicating a permission state
//! (granted / denied / prompt / unsupported) using a consistent
//! visual style across both the AV and Notifications sections.

use crate::i18n;
use leptos::prelude::*;
use leptos_i18n::t_string;

/// Normalised permission states used across the settings UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionState {
  Granted,
  Denied,
  Prompt,
  Unsupported,
}

impl PermissionState {
  /// Map a raw browser permission string to our enum.
  #[must_use]
  pub fn from_browser_str(s: &str) -> Self {
    match s {
      "granted" => Self::Granted,
      "denied" => Self::Denied,
      "default" | "prompt" => Self::Prompt,
      _ => Self::Unsupported,
    }
  }

  /// CSS modifier class for the badge.
  fn modifier(self) -> &'static str {
    match self {
      Self::Granted => "is-granted",
      Self::Denied => "is-denied",
      Self::Prompt => "is-prompt",
      Self::Unsupported => "is-unsupported",
    }
  }
}

/// Inline permission badge used by both AV and Notifications sections.
#[component]
pub fn PermissionBadge(state: PermissionState) -> impl IntoView {
  let i18n = i18n::use_i18n();
  let modifier = state.modifier();
  let label = match state {
    PermissionState::Granted => t_string!(i18n, settings.permission_granted),
    PermissionState::Denied => t_string!(i18n, settings.permission_denied),
    PermissionState::Prompt => t_string!(i18n, settings.permission_default),
    PermissionState::Unsupported => t_string!(i18n, settings.permission_unsupported),
  };
  view! {
    <span class=format!("settings-permission-badge {modifier}")>{label}</span>
  }
}
