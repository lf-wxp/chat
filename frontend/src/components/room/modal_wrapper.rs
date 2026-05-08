//! Shared modal wrapper used by all room dialogs (Sprint 4.1 of the
//! review-task-21 follow-up).
//!
//! Centralises the boilerplate every dialog needs:
//!
//! * Backdrop with `modal-backdrop-visible` styling.
//! * Outside-click dismissal (clicking the backdrop closes the modal).
//! * Escape-to-close keyboard shortcut.
//! * `role="dialog"` / `aria-modal="true"` accessibility attributes.
//! * Enter/exit CSS transitions driven by toggling `modal-backdrop-visible`.
//!
//! Components that opt into the wrapper provide just the inner content
//! via `children`. The wrapper is intentionally light-weight: it does
//! not attempt to manage focus traps but handles enter/exit animations
//! automatically.

use leptos::ev::keydown;
use leptos::prelude::*;
use leptos_use::{use_document, use_event_listener};

/// Available size presets. Maps to the existing `modal-{sm,md,lg}`
/// utility classes so styling stays consistent with the legacy modal
/// implementations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModalSize {
  /// Small (e.g. confirm dialog, password prompt).
  Small,
  /// Medium (default).
  #[default]
  Medium,
  /// Large (e.g. announcement editor).
  Large,
}

impl ModalSize {
  const fn css_modifier(self) -> &'static str {
    match self {
      Self::Small => " modal-sm",
      Self::Medium => "",
      Self::Large => " modal-lg",
    }
  }
}

/// Modal wrapper.
///
/// Consumers render their dialog markup as `children`. The wrapper
/// supplies the backdrop and dismissal handlers, plus automatic
/// enter/exit CSS transitions.
///
/// There are two usage modes:
///
/// 1. **Controlled** (preferred for animated modals): Pass an `open`
///    signal. The wrapper renders always but uses CSS to animate in/out.
///    When the exit animation completes, `on_close` fires.
///
/// 2. **Conditionally-mounted** (legacy): Wrap with `<Show when=...>`
///    and omit `open`. The wrapper animates only the entry.
#[component]
pub fn ModalWrapper(
  /// Callback invoked when the modal has fully closed (after exit
  /// animation completes). The parent should set its `open` signal
  /// to `false` here when using controlled mode.
  on_close: Callback<()>,
  /// Optional reactive open/close signal. When provided, the wrapper
  /// renders continuously and drives CSS transitions based on this
  /// value. When omitted, the wrapper renders immediately as visible
  /// and relies on the parent using `<Show>` for conditional mounting.
  #[prop(optional)]
  open: Option<Signal<bool>>,
  /// Size preset (small / medium / large).
  #[prop(optional)]
  size: ModalSize,
  /// Optional CSS class added to the inner dialog container so the
  /// caller can attach component-specific styles
  /// (e.g. `"announcement-editor"`).
  #[prop(into, optional)]
  class: Option<String>,
  /// Identifier of the dialog title element so screen readers can
  /// announce it. Required by `aria-labelledby`.
  #[prop(into)]
  labelled_by: String,
  /// `data-testid` applied to the dialog element. Defaults to
  /// `"modal-dialog"` if omitted.
  #[prop(into, optional)]
  testid: Option<String>,
  /// ARIA role. Defaults to `"dialog"`; override to `"alertdialog"`
  /// for confirmation dialogs that require immediate user attention.
  #[prop(into, optional)]
  dialog_role: Option<String>,
  /// Inner content rendered inside the dialog container.
  children: Children,
) -> impl IntoView {
  // Internal visible state that drives the CSS class.
  let visible = RwSignal::new(false);
  // Whether a close animation is in progress.
  let closing = RwSignal::new(false);
  // Whether the element has ever been shown (for conditional-mount mode).
  let mounted = RwSignal::new(false);

  let backdrop_ref: NodeRef<leptos::html::Div> = NodeRef::new();

  // --- Controlled mode: react to open signal changes ---
  if let Some(open_sig) = open {
    Effect::new(move |_| {
      let is_open = open_sig.get();
      if is_open {
        // Opening: mark mounted and schedule visible on next frame.
        mounted.set(true);
        closing.set(false);
        schedule_visible(visible);
      } else if visible.get_untracked() || mounted.get_untracked() {
        // Closing: start exit animation.
        start_exit_animation(visible, closing, mounted, backdrop_ref, on_close);
      }
    });
  } else {
    // --- Conditionally-mounted mode: animate entry immediately ---
    Effect::new(move |_| {
      if backdrop_ref.get().is_some() && !mounted.get_untracked() {
        mounted.set(true);
        schedule_visible(visible);
      }
    });
  }

  // Escape-to-close.
  let _ = use_event_listener(
    use_document(),
    keydown,
    move |ev: web_sys::KeyboardEvent| {
      if ev.key() == "Escape" && visible.get_untracked() && !closing.get_untracked() {
        ev.stop_propagation();
        if open.is_some() {
          // In controlled mode, trigger exit animation.
          start_exit_animation(visible, closing, mounted, backdrop_ref, on_close);
        } else {
          on_close.run(());
        }
      }
    },
  );

  let dialog_class = format!(
    "modal{}{}",
    size.css_modifier(),
    class.as_deref().map_or(String::new(), |c| format!(" {c}"))
  );
  let testid = testid.unwrap_or_else(|| "modal-dialog".to_string());
  let dialog_role = dialog_role.unwrap_or_else(|| "dialog".to_string());

  // In controlled mode, we need to decide whether to render at all.
  // We use `display: none` on the backdrop to hide it when not mounted.
  let should_display = move || {
    if open.is_some() { mounted.get() } else { true }
  };

  // Backdrop click handler.
  let handle_backdrop_click = move |_| {
    if closing.get_untracked() {
      return;
    }
    if open.is_some() {
      start_exit_animation(visible, closing, mounted, backdrop_ref, on_close);
    } else {
      on_close.run(());
    }
  };

  view! {
    <div
      node_ref=backdrop_ref
      class=move || {
        if visible.get() {
          "modal-backdrop modal-backdrop-visible"
        } else {
          "modal-backdrop"
        }
      }
      style=move || {
        if should_display() {
          ""
        } else {
          "display:none"
        }
      }
      role="presentation"
      data-testid="modal-wrapper-backdrop"
      on:click=handle_backdrop_click
    >
      <div
        class=dialog_class
        role=dialog_role
        aria-modal="true"
        aria-labelledby=labelled_by
        on:click=|ev| ev.stop_propagation()
        data-testid=testid
      >
        {children()}
      </div>
    </div>
  }
}

/// Schedule the `visible` signal to `true` after a short delay so the
/// browser can paint the initial (hidden) state first and the CSS
/// transition triggers.
fn schedule_visible(visible: RwSignal<bool>) {
  use wasm_bindgen::JsCast;
  use wasm_bindgen::prelude::*;

  let cb = Closure::once(move || {
    visible.set(true);
  });
  if let Some(win) = web_sys::window() {
    let _ =
      win.set_timeout_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), 20);
    cb.forget();
  }
}

/// Begin the exit animation: remove the visible class, then fire `on_close`
/// after the CSS transition ends (or a safety timeout).
fn start_exit_animation(
  visible: RwSignal<bool>,
  closing: RwSignal<bool>,
  mounted: RwSignal<bool>,
  backdrop_ref: NodeRef<leptos::html::Div>,
  on_close: Callback<()>,
) {
  if closing.get_untracked() {
    return; // already closing
  }
  closing.set(true);
  visible.set(false);

  use wasm_bindgen::JsCast;
  use wasm_bindgen::prelude::*;

  let fired = std::rc::Rc::new(std::cell::Cell::new(false));

  // Safety timeout — fire callback even if transitionend is swallowed.
  let fired_timeout = fired.clone();
  let timeout_closure = Closure::once(move || {
    if !fired_timeout.get() {
      fired_timeout.set(true);
      mounted.set(false);
      closing.set(false);
      on_close.run(());
    }
  });
  if let Some(win) = web_sys::window() {
    let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
      timeout_closure.as_ref().unchecked_ref(),
      350, // slightly longer than longest transition (300ms)
    );
  }
  timeout_closure.forget();

  // Primary path: fire on transitionend of the backdrop.
  if let Some(el) = backdrop_ref.get_untracked() {
    let fired_te = fired.clone();
    let te_closure = Closure::<dyn FnMut(web_sys::Event)>::new(move |_ev: web_sys::Event| {
      if !fired_te.get() {
        fired_te.set(true);
        mounted.set(false);
        closing.set(false);
        on_close.run(());
      }
    });
    let _ =
      el.add_event_listener_with_callback("transitionend", te_closure.as_ref().unchecked_ref());
    te_closure.forget();
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn css_modifier_small() {
    assert_eq!(ModalSize::Small.css_modifier(), " modal-sm");
  }

  #[test]
  fn css_modifier_medium_is_empty() {
    assert_eq!(ModalSize::Medium.css_modifier(), "");
  }

  #[test]
  fn css_modifier_large() {
    assert_eq!(ModalSize::Large.css_modifier(), " modal-lg");
  }

  #[test]
  fn default_size_is_medium() {
    assert_eq!(ModalSize::default(), ModalSize::Medium);
  }
}
