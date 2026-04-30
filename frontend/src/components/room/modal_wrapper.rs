//! Shared modal wrapper used by all room dialogs (Sprint 4.1 of the
//! review-task-21 follow-up).
//!
//! Centralises the boilerplate every dialog needs:
//!
//! * Backdrop with `modal-backdrop-visible` styling.
//! * Outside-click dismissal (clicking the backdrop closes the modal).
//! * Escape-to-close keyboard shortcut.
//! * `role="dialog"` / `aria-modal="true"` accessibility attributes.
//!
//! Components that opt into the wrapper provide just the inner content
//! via `children`. The wrapper is intentionally light-weight: it does
//! not attempt to manage focus traps or animations because each
//! consumer wants slightly different behaviour there.

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
/// supplies the backdrop and dismissal handlers.
#[component]
pub fn ModalWrapper(
  /// Callback invoked when the user dismisses the modal via the
  /// backdrop, the Escape key, or any other "close" gesture the
  /// consumer wires up. The parent is responsible for actually
  /// flipping the modal's `visible` state — this component does not
  /// own the visibility flag so it can be used by both controlled
  /// and conditionally-mounted modals.
  on_close: Callback<()>,
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
  // Escape-to-close. Use `stop_propagation()` on the event so that
  // when multiple modals are stacked (e.g. ConfirmDialog on top of
  // AnnouncementEditor), only the topmost modal's listener fires.
  // The inner (topmost) modal's listener runs first because its
  // event listener was registered later (higher in the DOM).
  let close_for_esc = on_close;
  let _ = use_event_listener(
    use_document(),
    keydown,
    move |ev: web_sys::KeyboardEvent| {
      if ev.key() == "Escape" {
        ev.stop_propagation();
        close_for_esc.run(());
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

  view! {
    <div
      class="modal-backdrop modal-backdrop-visible"
      role="presentation"
      data-testid="modal-wrapper-backdrop"
      on:click=move |_| on_close.run(())
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
