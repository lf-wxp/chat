//! Modal component

use leptos::prelude::*;
use leptos_i18n::t_string;

use crate::i18n::*;

/// Modal component
#[component]
pub fn Modal(
  /// Whether the modal is shown
  show: ReadSignal<bool>,
  /// Modal title
  #[prop(into)]
  title: String,
  /// Close callback
  on_close: Callback<()>,
  /// Child content
  children: Children,
) -> impl IntoView {
  let handle_overlay_click = move |_| {
    on_close.run(());
  };

  let handle_keydown = move |ev: web_sys::KeyboardEvent| {
    if ev.key() == "Escape" {
      on_close.run(());
    }
  };

  let i18n = use_i18n();
  let _children_view = children();
  let title_clone = title.clone();

  view! {
    <Show when=move || show.get()>
      <div
        class="modal-overlay"
        on:click=handle_overlay_click
        on:keydown=handle_keydown
        tabindex=-1
        role="dialog"
        aria-modal="true"
        aria-label=title_clone.clone()
      >
        <div class="modal-content" on:click=|ev| ev.stop_propagation()>
          <div class="modal-header">
            <h3 class="modal-title">{title.clone()}</h3>
            <button
              class="modal-close"
              on:click=move |_| on_close.run(())
              aria-label=t_string!(i18n, common_close)
              tabindex=0
            >
              "✕"
            </button>
          </div>
          <div class="modal-body">
          </div>
        </div>
      </div>
    </Show>
  }
}
