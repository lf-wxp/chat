//! Miscellaneous widgets
//!
//! Badge, Tooltip, EmptyState, ConnectionIndicator, DropdownItem, etc.

use leptos::prelude::*;

use crate::state;

/// Badge component (for unread counts, etc.)
#[component]
pub fn Badge(
  /// Count value
  count: u32,
) -> impl IntoView {
  if count == 0 {
    return {
      let _: () = view! {};
      ().into_any()
    };
  }

  let display = if count > 99 {
    "99+".to_string()
  } else {
    count.to_string()
  };

  view! {
    <span class="badge" aria-label=format!("{} unread", count)>
      {display}
    </span>
  }
  .into_any()
}

/// Tooltip component
#[component]
pub fn Tooltip(
  /// Tooltip text
  #[prop(into)]
  text: String,
  /// Child content
  children: Children,
) -> impl IntoView {
  let text_clone = text.clone();
  view! {
    <div class="tooltip-wrapper" aria-label=text_clone>
      {children()}
      <span class="tooltip-text">{text}</span>
    </div>
  }
}

/// Dropdown menu item
#[derive(Debug, Clone)]
pub struct DropdownItem {
  pub label: String,
  pub value: String,
  pub icon: Option<String>,
}

/// Empty state placeholder component
#[component]
pub fn EmptyState(
  /// Icon (emoji or text)
  #[prop(into)]
  icon: String,
  /// Title text
  #[prop(into)]
  title: String,
  /// Description text
  #[prop(into, optional)]
  description: String,
) -> impl IntoView {
  view! {
    <div class="empty-state">
      <div class="empty-icon">{icon}</div>
      <h3 class="empty-title">{title}</h3>
      {if description.is_empty() {
        let _: () = view! {};
        ().into_any()
      } else {
        view! { <p class="empty-description">{description}</p> }.into_any()
      }}
    </div>
  }
}

/// Connection status indicator
#[component]
pub fn ConnectionIndicator() -> impl IntoView {
  let conn_state = state::use_connection_state();

  view! {
    <div class="connection-indicator">
      {move || {
        let state = conn_state.get();
        let (class, label) = match state.ws_status {
          state::ConnectionStatus::Connected => ("status-connected", "Connected"),
          state::ConnectionStatus::Connecting => ("status-connecting", "Connecting..."),
          state::ConnectionStatus::Reconnecting => ("status-reconnecting", "Reconnecting..."),
          state::ConnectionStatus::Disconnected => ("status-disconnected", "Disconnected"),
        };
        view! {
          <span class=format!("status-dot {}", class) aria-label=label></span>
          <span class="status-text text-xs">{label}</span>
        }
      }}
    </div>
  }
}
