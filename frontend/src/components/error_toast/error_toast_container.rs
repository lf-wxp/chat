//! Error toast container component.
//!
//! Renders all active error toast notifications as a stack
//! in the bottom-right corner of the screen.

use super::error_toast_item::ErrorToastItem;
use crate::error_handler::use_error_toast_manager;
use leptos::prelude::*;

/// Error toast container component.
///
/// Displays all active error notifications as a stack.
/// Reactively updates when toasts are added or removed via RwSignal.
///
/// When the component unmounts (e.g. during page navigation), all
/// pending auto-remove timers are cancelled to prevent orphaned
/// closures and console warnings (W4 fix).
#[component]
pub fn ErrorToastContainer() -> impl IntoView {
  let manager = use_error_toast_manager();
  let toasts = manager.toasts_signal();

  on_cleanup(move || {
    manager.clear_all();
  });

  view! {
    <div class="error-toast-container" aria-live="polite" aria-atomic="false">
      <For
        each=move || toasts.get()
        key=|toast| toast.id
        children=move |toast| {
          view! {
            <ErrorToastItem toast={toast} />
          }
        }
      />
    </div>
  }
}
