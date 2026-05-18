//! Context menu (Pin / Mute / Archive) for a sidebar conversation row.
//!
//! The menu is rendered as a popover anchored to the row's more-actions
//! button. It closes automatically on Escape, on outside pointer down,
//! and after any action is invoked (Req 7.7, Req 14.5.2).
//!
//! The conversation state mutations themselves live on [`AppState`] so
//! the menu is purely presentational and can be unit-tested against the
//! same toggle methods used elsewhere in the app.

use crate::i18n;
use crate::state::{ConversationId, MAX_PINS, use_app_state};
use icondata as i;
use leptos::ev;
use leptos::prelude::*;
use leptos_i18n::t_string;
use leptos_icons::Icon;
use leptos_use::use_event_listener;
use wasm_bindgen::JsCast;
use web_sys::Element;

/// Returns whether an event target is nested inside the menu popover
/// or its trigger button. Exposed as a pure function so the outside-
/// click heuristic can be exercised without a DOM (the parameter
/// emulates the closest-matching selector probe).
#[must_use]
pub fn is_inside_menu_chrome(inside_menu: bool, on_trigger: bool) -> bool {
  inside_menu || on_trigger
}

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
#[component]
pub fn SidebarConversationMenu(
  conversation_id: ConversationId,
  pinned: Signal<bool>,
  muted: Signal<bool>,
  archived: Signal<bool>,
  open: RwSignal<bool>,
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

  // Close the menu on Escape. `stop_propagation()` prevents the
  // global Escape handler (app.rs) from also firing — without it,
  // a modal underneath would close simultaneously (review §3.1).
  let _ = use_event_listener(
    leptos_use::use_window(),
    ev::keydown,
    move |ev: web_sys::KeyboardEvent| {
      if crate::utils::safe_key(&ev) == "Escape" && open.get_untracked() {
        open.set(false);
        ev.stop_propagation();
      }
    },
  );

  // Close on outside pointer-down. We inspect the event target to let
  // clicks on the menu itself or the trigger button pass through —
  // the trigger is handled by the parent row and would otherwise flap
  // the menu open-and-closed on a single click.
  let _ = use_event_listener(
    leptos_use::use_window(),
    ev::pointerdown,
    move |ev: web_sys::PointerEvent| {
      if !open.get_untracked() {
        return;
      }
      let Some(target) = ev.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
        return;
      };
      let inside_menu = target
        .closest(".sidebar-conversation-menu")
        .ok()
        .flatten()
        .is_some();
      let on_trigger = target
        .closest(".sidebar-conversation-actions-btn")
        .ok()
        .flatten()
        .is_some();
      if !is_inside_menu_chrome(inside_menu, on_trigger) {
        open.set(false);
      }
    },
  );

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

  view! {
    <div
      class="sidebar-conversation-menu"
      role="menu"
      aria-orientation="vertical"
      on:click=move |ev: ev::MouseEvent| ev.stop_propagation()
      data-testid="sidebar-conversation-menu"
    >
      <button
        type="button"
        class="sidebar-conversation-menu__item"
        role="menuitem"
        aria-label=pin_label
        aria-disabled=move || if pin_at_cap.get() { "true" } else { "false" }
        data-testid="sidebar-conversation-menu-pin"
        title=move || {
          if pin_at_cap.get() {
            t_string!(i18n, sidebar.pin_limit_reached)
          } else {
            pin_label()
          }
        }
        on:click={
          let id = id_for_pin;
          move |_| {
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
          }
        }
      >
        <Icon icon=i::LuPin />
        <span>{pin_label}</span>
      </button>

      <button
        type="button"
        class="sidebar-conversation-menu__item"
        role="menuitem"
        aria-label=mute_label
        title=mute_label
        data-testid="sidebar-conversation-menu-mute"
        on:click={
          let id = id_for_mute;
          move |_| {
            app_state.toggle_mute(&id);
            open.set(false);
          }
        }
      >
        <Icon icon=move || if muted.get() { i::LuBell } else { i::LuBellOff } />
        <span>{mute_label}</span>
      </button>

      <button
        type="button"
        class="sidebar-conversation-menu__item"
        role="menuitem"
        aria-label=archive_label
        title=archive_label
        data-testid="sidebar-conversation-menu-archive"
        on:click={
          let id = id_for_archive;
          move |_| {
            app_state.toggle_archive(&id);
            open.set(false);
          }
        }
      >
        <Icon icon=i::LuArchive />
        <span>{archive_label}</span>
      </button>
    </div>
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn outside_click_detector_respects_menu_chrome() {
    assert!(is_inside_menu_chrome(true, false));
    assert!(is_inside_menu_chrome(false, true));
    assert!(is_inside_menu_chrome(true, true));
    assert!(!is_inside_menu_chrome(false, false));
  }

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
