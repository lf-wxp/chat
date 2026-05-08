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
///
/// Accepts a reactive `Signal<PermissionState>` so the label and
/// CSS modifier update when either the browser permission changes
/// or the user switches locale. Wrapping the `t_string!` call in a
/// reactive closure is required by `leptos_i18n` — accessing the
/// Locale signal outside a tracking context emits a warning and
/// prevents the label from re-rendering on language switch.
#[component]
pub fn PermissionBadge(#[prop(into)] state: Signal<PermissionState>) -> impl IntoView {
  let i18n = i18n::use_i18n();
  let badge_class = move || format!("settings-permission-badge {}", state.get().modifier());
  let label = move || match state.get() {
    PermissionState::Granted => t_string!(i18n, settings.permission_granted),
    PermissionState::Denied => t_string!(i18n, settings.permission_denied),
    PermissionState::Prompt => t_string!(i18n, settings.permission_default),
    PermissionState::Unsupported => t_string!(i18n, settings.permission_unsupported),
  };
  view! { <span class=badge_class>{label}</span> }
}
