//! Shared popover wrapper for non-modal floating elements (context
//! menus, dropdowns, etc.) that need Escape-to-close and
//! outside-click-to-close behaviour.
//!
//! Unlike [`super::modal_wrapper::ModalWrapper`] this does **not**
//! render a backdrop or trap focus — it only attaches the dismissal
//! event listeners so callers can focus on content.

use leptos::ev::{keydown, mousedown};
use leptos::prelude::*;
use leptos_use::{use_document, use_event_listener};
use wasm_bindgen::JsCast;

/// Popover wrapper.
///
/// Wraps a floating element (e.g. a context menu) and provides:
/// * Escape key dismissal.
/// * Outside-mousedown dismissal.
///
/// The caller is responsible for rendering the element positioned
/// absolutely relative to its anchor.
#[component]
pub fn PopoverWrapper(
  /// Callback invoked when the popover should close.
  on_close: Callback<()>,
  /// Inner content rendered inside the popover.
  children: Children,
) -> impl IntoView {
  let node_ref: NodeRef<leptos::html::Div> = NodeRef::new();

  // Escape-to-close.
  let close_for_esc = on_close;
  let _ = use_event_listener(
    use_document(),
    keydown,
    move |ev: web_sys::KeyboardEvent| {
      if ev.key() == "Escape" {
        close_for_esc.run(());
      }
    },
  );

  // Outside-mousedown-to-close.
  let close_for_outside = on_close;
  let _ = use_event_listener(use_document(), mousedown, move |ev: web_sys::MouseEvent| {
    let Some(el) = node_ref.get() else {
      return;
    };
    if let Some(target) = ev.target().and_then(|t| t.dyn_into::<web_sys::Node>().ok())
      && !el.contains(Some(&target))
    {
      close_for_outside.run(());
    }
  });

  view! {
    <div node_ref=node_ref on:click=|ev| ev.stop_propagation()>
      {children()}
    </div>
  }
}
