//! Lightweight modal dialog for confirm / alert interactions.
//!
//! Replaces `window.confirm()` / `window.alert()` with styled
//! Leptos components that support dark mode and are accessible
//! (P2-7 from code review).
//!
//! P2-D fix: the `confirm()` future is now event-driven via a
//! `futures::channel::oneshot` — a button click resolves the
//! future immediately instead of waiting for the next 50 ms poll
//! tick.

use crate::i18n;
use icondata as i;
use leptos::prelude::*;
use leptos_i18n::t_string;
use leptos_icons::Icon;
use std::cell::RefCell;
use std::rc::Rc;

/// Reactive state driving the modal. Only one modal can be visible
/// at a time (last one wins if multiple are queued).
#[derive(Debug, Clone)]
pub struct DialogState {
  /// Currently visible dialog, if any.
  visible: RwSignal<bool>,
  /// Message text.
  message: RwSignal<String>,
  /// Whether to show the Cancel button (confirm mode vs alert mode).
  show_cancel: RwSignal<bool>,
  /// Pending confirm resolver. Set by [`DialogState::confirm`]
  /// before the modal is shown and consumed by the first
  /// `on_ok`/`on_cancel` click.
  pending: Rc<RefCell<Option<futures::channel::oneshot::Sender<bool>>>>,
}

impl Default for DialogState {
  fn default() -> Self {
    Self::new()
  }
}

crate::wasm_send_sync!(DialogState);

impl DialogState {
  /// Create a fresh (hidden) dialog state.
  #[must_use]
  pub fn new() -> Self {
    Self {
      visible: RwSignal::new(false),
      message: RwSignal::new(String::new()),
      show_cancel: RwSignal::new(false),
      pending: Rc::new(RefCell::new(None)),
    }
  }

  /// Show an alert (single "OK" button).
  pub fn alert(&self, msg: impl Into<String>) {
    self.message.set(msg.into());
    self.show_cancel.set(false);
    // Drop any pending confirm resolver so a straggler OK click on
    // the alert dialog does not accidentally resolve a stale
    // confirm future with `true`.
    self.pending.borrow_mut().take();
    self.visible.set(true);
  }

  /// Show a confirm dialog (OK + Cancel). Returns `true` if the
  /// user clicks OK, `false` otherwise (or if the dialog is
  /// re-opened for another purpose before the user responds).
  pub async fn confirm(&self, msg: impl Into<String>) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
      let _ = msg;
      panic!(
        "DialogState.confirm() is not supported in native tests; mock the dialog result instead"
      );
    }
    #[cfg(target_arch = "wasm32")]
    {
      let (tx, rx) = futures::channel::oneshot::channel();
      // Replacing an in-flight pending resolver is fine — the old
      // one is dropped (the old future resolves to `false`), then
      // the new confirm takes over.
      if let Some(prev) = self.pending.borrow_mut().replace(tx) {
        drop(prev);
      }
      self.message.set(msg.into());
      self.show_cancel.set(true);
      self.visible.set(true);

      // Await the button click (or channel drop = `false`).
      rx.await.unwrap_or(false)
    }
  }

  /// Resolve the pending confirm with the given result and close
  /// the dialog. Called from the OK/Cancel button handlers.
  fn resolve(&self, result: bool) {
    if let Some(tx) = self.pending.borrow_mut().take() {
      // Ignore send errors — they just mean the caller already
      // dropped the future.
      let _ = tx.send(result);
    }
    self.visible.set(false);
  }
}

/// Modal dialog component.
///
/// Mount this once in the chat view root. It reads from a shared
/// `DialogState` and renders a styled overlay when active.
#[component]
pub fn Dialog(
  /// Shared dialog state.
  state: DialogState,
) -> impl IntoView {
  let i18n = i18n::use_i18n();

  let state_ok = state.clone();
  let state_cancel = state.clone();
  let on_ok: Callback<()> = Callback::new(move |_: ()| state_ok.resolve(true));
  let on_cancel: Callback<()> = Callback::new(move |_: ()| state_cancel.resolve(false));

  view! {
    <Show when=move || state.visible.get() fallback=|| ()>
      <div class="dialog-overlay">
        <div class="dialog-box" role="dialog" aria-modal="true">
          <p class="dialog-message">{move || state.message.get()}</p>
          <div class="dialog-actions">
            <Show when=move || state.show_cancel.get() fallback=|| ()>
              <button
                type="button"
                class="dialog-btn dialog-btn-cancel"
                aria-label=move || t_string!(i18n, common.cancel)
                title=move || t_string!(i18n, common.cancel)
                on:click=move |_| on_cancel.run(())
              >
                <Icon icon=i::LuX />
              </button>
            </Show>
            <button
              type="button"
              class="dialog-btn dialog-btn-ok"
              aria-label=move || t_string!(i18n, common.ok)
              title=move || t_string!(i18n, common.ok)
              on:click=move |_| on_ok.run(())
            >
              <Icon icon=i::LuCheck />
            </button>
          </div>
        </div>
      </div>
    </Show>
  }
}
