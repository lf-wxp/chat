//! Context menu (Pin / Mute / Archive) for a sidebar conversation row.
//!
//! The menu is rendered as a popover anchored to the row's more-actions
//! button. It closes automatically on Escape, on outside pointer down,
//! and after any action is invoked (Req 7.7, Req 14.5.2).
//!
//! The conversation state mutations themselves live on [`AppState`] so
//! the menu is purely presentational and can be unit-tested against the
//! same toggle methods used elsewhere in the app.
//!
//! Positioning / dismissal are provided by the generic
//! [`DropdownMenu`] component; this file only owns the conversation-
//! specific action wiring.

use crate::components::dropdown_menu::{DropdownMenu, DropdownMenuItem};
use crate::i18n;
use crate::state::{ConversationId, MAX_PINS, use_app_state};
use icondata as i;
use leptos::html;
use leptos::prelude::*;
use leptos_i18n::t_string;
use leptos_icons::Icon;

/// Decision emitted by [`pin_click_action`] for the click handler on
/// the "Pin" menu item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinClickAction {
  /// Currently pinned → unpin (always allowed).
  Unpin,
  /// Currently unpinned and below the cap → pin.
  Pin,
  /// Currently unpinned but the cap is reached → show the limit
  /// toast and leave the pin state alone (Req 7.7c).
  ShowLimitToast,
}

/// Pure decision helper for the "Pin" click handler. Splits the
/// branchy logic (currently pinned / at cap / below cap) into a
/// testable function so the toast / toggle behaviour cannot regress
/// silently (review v3 §T3).
#[must_use]
pub fn pin_click_action(currently_pinned: bool, at_cap: bool) -> PinClickAction {
  if currently_pinned {
    PinClickAction::Unpin
  } else if at_cap {
    PinClickAction::ShowLimitToast
  } else {
    PinClickAction::Pin
  }
}

/// Conversation context menu.
///
/// * `conversation_id` — which conversation the menu is acting on.
/// * `pinned` / `muted` / `archived` — current flags as reactive
///   signals (used to pick the right label, e.g. "Pin" vs. "Unpin").
///   They are signals, not plain bools, because the parent row does
///   NOT re-mount on flag toggles — the menu has to read fresh
///   values when the user re-opens it after a previous toggle (G22
///   in the e2e coverage plan).
/// * `open` — externally controlled visibility signal; the menu itself
///   flips this to `false` on close.
/// * `on_delete` — called when the user clicks the Delete item.
///   The parent owns the confirmation flow (via its own
///   `ModalWrapper`) so the menu can stay decoupled from any global
///   `DialogState` and remains usable from sidebar paths that mount
///   without an active chat view (G21).
/// * `trigger` — reference to the more-actions button so the generic
///   [`DropdownMenu`] can compute its fixed-position coordinates.
#[component]
pub fn SidebarConversationMenu(
  conversation_id: ConversationId,
  pinned: Signal<bool>,
  muted: Signal<bool>,
  archived: Signal<bool>,
  open: RwSignal<bool>,
  on_delete: Callback<()>,
  trigger: NodeRef<html::Button>,
) -> impl IntoView {
  let app_state = use_app_state();
  let i18n = i18n::use_i18n();
  // Toast manager — used to surface a friendly "Pin limit reached"
  // message when the user attempts to pin a 6th conversation
  // (Req 7.7c). Resolves to `None` outside the Leptos tree so the
  // menu component can still be unit-tested.
  let toast = use_context::<crate::error_handler::ErrorToastManager>();

  // Whether pinning is currently at or above the cap. The actual
  // toggle action keeps the button enabled — clicking it surfaces a
  // toast instead of silently no-op'ing — so we use this only for the
  // tooltip / aria hint.
  let pin_at_cap = Memo::new(move |_| {
    if pinned.get() {
      false
    } else {
      app_state
        .conversations
        .with(|list| list.iter().filter(|c| c.pinned).count() >= MAX_PINS)
    }
  });

  let id_for_pin = conversation_id.clone();
  let id_for_mute = conversation_id.clone();
  let id_for_archive = conversation_id.clone();

  let pin_label = move || {
    if pinned.get() {
      t_string!(i18n, sidebar.unpin)
    } else {
      t_string!(i18n, sidebar.pin)
    }
  };
  let mute_label = move || {
    if muted.get() {
      t_string!(i18n, sidebar.unmute)
    } else {
      t_string!(i18n, sidebar.mute)
    }
  };
  let archive_label = move || {
    if archived.get() {
      t_string!(i18n, sidebar.unarchive)
    } else {
      t_string!(i18n, sidebar.archive)
    }
  };
  let delete_label = move || t_string!(i18n, sidebar.delete);

  view! {
    <DropdownMenu
      open=open
      trigger=trigger
      estimated_items=4
      testid="sidebar-conversation-menu"
    >
      <DropdownMenuItem
        aria_label=pin_label()
        title=move || {
          if pin_at_cap.get() {
            t_string!(i18n, sidebar.pin_limit_reached)
          } else {
            pin_label()
          }
        }
        testid="sidebar-conversation-menu-pin"
        on_click={
          let id = id_for_pin;
          Callback::new(move |_: ()| {
            // Req 7.7c — drive the user-visible feedback through the
            // pure `pin_click_action` decision helper so the toast /
            // toggle behaviour is unit-testable in isolation.
            match pin_click_action(pinned.get_untracked(), pin_at_cap.get_untracked()) {
              PinClickAction::ShowLimitToast => {
                if let Some(toast) = toast {
                  toast.show_info_message_with_key(
                    "SYS501",
                    "sidebar.pin_limit_reached",
                    "",
                  );
                }
              }
              PinClickAction::Pin | PinClickAction::Unpin => {
                app_state.toggle_pin(&id);
              }
            }
            open.set(false);
          })
        }
      >
        <Icon icon=i::LuPin />
        <span>{pin_label}</span>
      </DropdownMenuItem>

      <DropdownMenuItem
        aria_label=mute_label()
        title=mute_label()
        testid="sidebar-conversation-menu-mute"
        on_click={
          let id = id_for_mute;
          Callback::new(move |_: ()| {
            app_state.toggle_mute(&id);
            open.set(false);
          })
        }
      >
        <Icon icon=move || if muted.get() { i::LuBell } else { i::LuBellOff } />
        <span>{mute_label}</span>
      </DropdownMenuItem>

      <DropdownMenuItem
        aria_label=archive_label()
        title=archive_label()
        testid="sidebar-conversation-menu-archive"
        on_click={
          let id = id_for_archive;
          Callback::new(move |_: ()| {
            app_state.toggle_archive(&id);
            open.set(false);
          })
        }
      >
        <Icon icon=i::LuArchive />
        <span>{archive_label}</span>
      </DropdownMenuItem>

      // G21 — Delete conversation. Closes the menu first so the
      // outside-click listener does not race the modal that the
      // parent will open.
      <DropdownMenuItem
        danger=true
        aria_label=delete_label()
        title=delete_label()
        testid="sidebar-conversation-menu-delete"
        on_click=Callback::new(move |_: ()| {
          open.set(false);
          on_delete.run(());
        })
      >
        <Icon icon=i::LuTrash2 />
        <span>{delete_label}</span>
      </DropdownMenuItem>
    </DropdownMenu>
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // ── T3: pin click decision matrix ──

  #[test]
  fn pin_click_unpin_when_already_pinned() {
    // Currently pinned — unpin always allowed regardless of cap.
    assert_eq!(pin_click_action(true, false), PinClickAction::Unpin);
    assert_eq!(pin_click_action(true, true), PinClickAction::Unpin);
  }

  #[test]
  fn pin_click_pins_when_not_at_cap() {
    assert_eq!(pin_click_action(false, false), PinClickAction::Pin);
  }

  #[test]
  fn pin_click_emits_toast_when_at_cap() {
    // Req 7.7c — the 6th attempt must surface the limit toast,
    // not silently no-op or disable the button.
    assert_eq!(
      pin_click_action(false, true),
      PinClickAction::ShowLimitToast,
    );
  }
}
