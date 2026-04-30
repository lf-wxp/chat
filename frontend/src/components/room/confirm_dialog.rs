//! Inline confirmation dialog for destructive room-management actions
//! such as kick / ban / transfer ownership (Req 15.6 §47).
//!
//! The dialog is modal, keyboard-dismissable via Escape, and focuses
//! the primary action button on open so keyboard users can confirm
//! immediately without tabbing.

use icondata as i;
use leptos::prelude::*;
use leptos_i18n::t_string;
use leptos_icons::Icon;

use crate::components::room::modal_wrapper::{ModalSize, ModalWrapper};
use crate::i18n;

/// Accent applied to the primary action button. Destructive actions
/// get a red-coloured button to reinforce the warning semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmTone {
  /// Irreversible or user-facing destructive action (kick / ban).
  Destructive,
  /// Non-destructive confirmation (e.g. promote / demote).
  Neutral,
}

/// Modal confirmation dialog with a title, descriptive body, and two
/// buttons. Mount conditionally via `<Show when=...>`; the component
/// unconditionally renders its overlay when invoked.
#[component]
#[allow(clippy::needless_pass_by_value)]
pub fn ConfirmDialog(
  /// Dialog title text.
  #[prop(into)]
  title: Signal<String>,
  /// Body description; keep this short (one sentence).
  #[prop(into)]
  description: Signal<String>,
  /// Text for the primary (confirm) button.
  #[prop(into)]
  confirm_label: Signal<String>,
  /// Styling applied to the confirm button.
  #[prop(optional)]
  tone: Option<Signal<ConfirmTone>>,
  /// Invoked when the user clicks the confirm button.
  on_confirm: Callback<()>,
  /// Invoked when the user clicks cancel, presses Escape, or clicks
  /// outside the dialog.
  on_cancel: Callback<()>,
) -> impl IntoView {
  let i18n = i18n::use_i18n();
  let tone_signal = tone.unwrap_or(Signal::derive(|| ConfirmTone::Destructive));

  // Auto-focus the primary action for keyboard-first confirmation.
  let confirm_ref: NodeRef<leptos::html::Button> = NodeRef::new();
  Effect::new(move |_| {
    if let Some(btn) = confirm_ref.get() {
      let _ = btn.focus();
    }
  });

  view! {
    <ModalWrapper
      on_close=on_cancel
      size=ModalSize::Small
      class="room-confirm-dialog"
      labelled_by="room-confirm-title"
      testid="room-confirm-dialog"
      dialog_role="alertdialog"
    >
      <div aria-describedby="room-confirm-desc">
        <header class="modal-header">
          <h2 id="room-confirm-title" class="modal-title">{move || title.get()}</h2>
        </header>
        <div class="modal-body">
          <p id="room-confirm-desc" class="room-confirm-dialog__body">
            {move || description.get()}
          </p>
        </div>
        <footer class="modal-footer">
          <button
            type="button"
            class="dialog-btn dialog-btn-cancel"
            aria-label=move || t_string!(i18n, common.cancel)
            title=move || t_string!(i18n, common.cancel)
            on:click=move |_| on_cancel.run(())
            data-testid="room-confirm-cancel"
          >
            <Icon icon=i::LuX />
          </button>
          <button
            type="button"
            class=move || {
              let base = "dialog-btn";
              match tone_signal.get() {
                ConfirmTone::Destructive => format!("{base} dialog-btn-danger"),
                ConfirmTone::Neutral => format!("{base} dialog-btn-ok"),
              }
            }
            node_ref=confirm_ref
            aria-label=move || confirm_label.get()
            title=move || confirm_label.get()
            on:click=move |_| on_confirm.run(())
            data-testid="room-confirm-ok"
          >
            <Icon icon=i::LuCheck />
          </button>
        </footer>
      </div>
    </ModalWrapper>
  }
}
