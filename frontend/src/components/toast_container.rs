//! Toast container component.

use leptos::prelude::*;

/// Toast container component.
#[component]
pub fn ToastContainer() -> impl IntoView {
  view! {
    <div class="toast-container-bottom-right" aria-live="polite" aria-atomic="true" />
  }
}
