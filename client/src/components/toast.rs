//! Toast notification component

use leptos::prelude::*;

use crate::state;

/// Toast container component
#[component]
pub fn ToastContainer() -> impl IntoView {
  let ui_state = state::use_ui_state();

  view! {
    <div class="toast-container" aria-live="polite">
      {move || {
        ui_state.get().toasts.iter().map(|toast| {
          let type_class = match toast.toast_type {
            state::ToastType::Success => "toast-success",
            state::ToastType::Error => "toast-error",
            state::ToastType::Warning => "toast-warning",
            state::ToastType::Info => "toast-info",
          };
          let msg = toast.message.clone();
          view! {
            <div class=format!("toast {}", type_class)>
              <span class="toast-message">{msg}</span>
            </div>
          }
        }).collect_view()
      }}
    </div>
  }
}
