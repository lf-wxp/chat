//! Sidebar conversation item component.
//!
//! Renders a single row in the pinned / active / archived sections of
//! the sidebar plus the Pin / Mute / Archive / Delete context menu
//! used to mutate its flags (Req 7.7, Req 14.1.2). The Delete action
//! opens an inline `ModalWrapper` confirmation owned by this row —
//! the modal does not depend on `DialogState` so deletion remains
//! reachable when no chat view is mounted (G21).

use super::sidebar_conversation_menu::SidebarConversationMenu;
use crate::components::room::modal_wrapper::{ModalSize, ModalWrapper};
use crate::i18n;
use crate::state::use_app_state;
use icondata as i;
use leptos::ev;
use leptos::prelude::*;
use leptos_i18n::t_string;
use leptos_icons::Icon;
use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlElement};

/// Pure helper: pick the next focus index given the key pressed and
/// the current row's index in the focusable-row list.
///
/// Wraps around at both ends so users can cycle through the list
/// indefinitely. Returns `None` when the keyboard event is not a
/// supported list-navigation key — callers can short-circuit
/// rendering without an extra match.
#[must_use]
pub fn next_focus_index(key: &str, current: usize, count: usize) -> Option<usize> {
  if count == 0 {
    return None;
  }
  match key {
    "ArrowDown" => Some((current + 1) % count),
    "ArrowUp" => Some((current + count - 1) % count),
    "Home" => Some(0),
    "End" => Some(count - 1),
    _ => None,
  }
}

/// Move focus from `current` to the previous / next sibling row in
/// the same sidebar tree, wrapping around at the edges. Looks up all
/// elements that match `[data-testid="sidebar-conversation-item"]`
/// inside the closest `aside.sidebar` ancestor so all three sections
/// (Pinned / Active / Archived) participate in a single roving tab
/// order (Req 14.5.2).
fn focus_sibling_conversation(current: &Element, key: &str) {
  let Some(root) = current.closest("aside.sidebar").ok().flatten() else {
    return;
  };
  let Ok(rows) = root.query_selector_all("[data-testid=\"sidebar-conversation-item\"]") else {
    return;
  };
  let count = rows.length() as usize;
  if count == 0 {
    return;
  }
  // Locate the current row's index.
  let mut current_idx = 0usize;
  for i in 0..count {
    if let Some(node) = rows.item(i as u32)
      && node.is_same_node(Some(current))
    {
      current_idx = i;
      break;
    }
  }
  let Some(target_idx) = next_focus_index(key, current_idx, count) else {
    return;
  };
  if let Some(node) = rows.item(target_idx as u32)
    && let Ok(el) = node.dyn_into::<HtmlElement>()
  {
    let _ = el.focus();
  }
}

/// Sidebar conversation item component.
#[component]
pub fn SidebarConversationItem(conversation: crate::state::Conversation) -> impl IntoView {
  let app_state = use_app_state();
  let i18n = i18n::use_i18n();
  let conv_id = conversation.id.clone();
  let display_name = conversation.display_name.clone();
  let first_char = display_name.chars().next().unwrap_or('?');

  // Stable identifier strings for E2E selectors. `data-room-id` is
  // populated only when the conversation is a Room; direct
  // conversations get `data-conversation-type="direct"` instead so a
  // single locator can target either kind without splitting on text.
  let (data_room_id, data_conv_type) = match &conv_id {
    crate::state::ConversationId::Room(rid) => (Some(rid.to_string()), "room".to_string()),
    crate::state::ConversationId::Direct(_) => (None, "direct".to_string()),
  };

  // Reactive snapshot of the *current* conversation row from the
  // global signal. The component is keyed by `conversation.id` in
  // the parent `<For>`, so it does NOT re-mount when the underlying
  // `Conversation` value changes (e.g. mute/pin toggles, last-
  // message updates). Reading the prop fields directly would freeze
  // the row on the first-render snapshot — `last_message_preview`
  // already worked around this; we extend the same treatment to
  // every flag the row renders so toggles are reflected live without
  // a section reparent.
  //
  // ⚠️ History — review G22 in `e2e-coverage-plan.md` §2.1: capturing
  // `pinned` / `muted` / `archived` / `unread_count` from the prop
  // by value made the mute action a UI no-op until reload (the row
  // doesn't change section, so `<For>` keeps the stale instance
  // alive). All flag-derived attributes / classes / Show gates
  // below now go through these signals.
  let conv_id_for_lookup = conv_id.clone();
  let lookup = Signal::derive(move || {
    app_state
      .conversations
      .get()
      .into_iter()
      .find(|c| c.id == conv_id_for_lookup)
  });

  let pinned_sig = Signal::derive(move || lookup.get().is_some_and(|c| c.pinned));
  let muted_sig = Signal::derive(move || lookup.get().is_some_and(|c| c.muted));
  let archived_sig = Signal::derive(move || lookup.get().is_some_and(|c| c.archived));
  let unread_count_sig = Signal::derive(move || lookup.get().map(|c| c.unread_count).unwrap_or(0));
  let last_message_preview = Signal::derive(move || {
    lookup
      .get()
      .and_then(|c| c.last_message)
      .unwrap_or_default()
  });

  let conv_id_active = conv_id.clone();
  let is_active =
    Signal::derive(move || app_state.active_conversation.get() == Some(conv_id_active.clone()));

  let menu_open = RwSignal::new(false);
  // G21 — delete confirmation modal. Local to this row so the
  // dialog is reachable even when there is no active chat view.
  let delete_modal_open = RwSignal::new(false);
  let delete_modal_open_signal: Signal<bool> = delete_modal_open.into();

  let has_unread = Signal::derive(move || unread_count_sig.get() > 0u32);

  let item_class = move || {
    let mut classes = String::from("sidebar-conversation");
    if is_active.get() {
      classes.push_str(" sidebar-item-active");
    } else {
      classes.push_str(" sidebar-item");
    }
    if muted_sig.get() {
      classes.push_str(" sidebar-conversation-muted");
    }
    if pinned_sig.get() {
      classes.push_str(" sidebar-conversation-pinned");
    }
    if menu_open.get() {
      classes.push_str(" has-menu-open");
    }
    classes
  };

  // Accessible label for the row: includes display name, muted
  // status and unread count so screen readers announce new activity
  // *and* the per-conversation Do-Not-Disturb mode (Req 7.7e — the
  // mute icon is decorative-only so we surface the meaning textually
  // for assistive tech). The muted/unread suffix words go through
  // i18n so non-English users hear the localised label (review v3
  // §O2).
  let aria_label = {
    let name = display_name.clone();
    move || {
      let mut parts = vec![name.clone()];
      if muted_sig.get() {
        parts.push(t_string!(i18n, sidebar.muted_aria_suffix).to_string());
      }
      let count = unread_count_sig.get();
      if count > 0 {
        parts.push(format!(
          "{count} {}",
          t_string!(i18n, sidebar.unread_aria_suffix)
        ));
      }
      parts.join(" — ")
    }
  };

  let conv_id_menu = conv_id.clone();

  view! {
    <div
      class=item_class
      tabindex="0"
      role="button"
      aria-label=aria_label
      aria-pressed=move || if is_active.get() { "true" } else { "false" }
      on:click={
        let conv_id_click = conv_id.clone();
        move |_| {
          app_state.active_conversation.set(Some(conv_id_click.clone()));
          // On mobile, hide the sidebar so the chat view gets the
          // full viewport width. The top-bar back button restores it.
          app_state.sidebar_visible.set(false);
        }
      }
      on:keydown={
        let conv_id_key = conv_id.clone();
        move |ev: web_sys::KeyboardEvent| {
          let key = crate::utils::safe_key(&ev);
          // Activate row.
          if key == "Enter" || key == " " {
            ev.prevent_default();
            app_state.active_conversation.set(Some(conv_id_key.clone()));
            app_state.sidebar_visible.set(false);
            return;
          }
          // Arrow / Home / End — list navigation per Req 14.5.2.
          if matches!(key.as_str(), "ArrowDown" | "ArrowUp" | "Home" | "End")
            && let Some(target) = ev
              .current_target()
              .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
          {
            ev.prevent_default();
            focus_sibling_conversation(&target, key.as_str());
          }
        }
      }
      data-testid="sidebar-conversation-item"
      data-conversation-type=data_conv_type.clone()
      data-room-id=data_room_id.clone().unwrap_or_default()
      data-pinned=move || if pinned_sig.get() { "true" } else { "false" }
      data-archived=move || if archived_sig.get() { "true" } else { "false" }
      data-muted=move || if muted_sig.get() { "true" } else { "false" }
    >
      // Avatar
      <div class="sidebar-conversation-avatar" aria-hidden="true">
        <div class="avatar avatar-sm">
          <span class="text-sm font-semibold">
            {first_char.to_string()}
          </span>
        </div>
      </div>

      // Conversation info
      <div class="sidebar-conversation-info">
        <div class="sidebar-conversation-name truncate">
          <Show when=move || pinned_sig.get()>
            <Icon icon=i::LuPin attr:class="sidebar-conversation-pin-icon" />
          </Show>
          <span>{display_name.clone()}</span>
        </div>
        <div class="sidebar-conversation-preview truncate">
          {move || last_message_preview.get()}
        </div>
      </div>

      // Unread badge — aria-live="polite" announces count changes to
      // screen readers without being overly intrusive (WCAG 4.1.3).
      <Show when=move || has_unread.get()>
        <span class="sidebar-item-badge-unread" aria-live="polite">
          {move || unread_count_sig.get()}
        </span>
      </Show>

      // Mute indicator (decorative — label is baked into aria-label)
      <Show when=move || muted_sig.get()>
        <span class="sidebar-conversation-mute-icon" aria-hidden="true">
          <Icon icon=i::LuBellOff />
        </span>
      </Show>

      // Context menu trigger
      <button
        type="button"
        class="sidebar-conversation-actions-btn"
        aria-haspopup="menu"
        aria-expanded=move || if menu_open.get() { "true" } else { "false" }
        aria-label=move || t_string!(i18n, sidebar.open_conversation_actions)
        title=move || t_string!(i18n, common.more)
        on:click=move |ev: ev::MouseEvent| {
          ev.stop_propagation();
          menu_open.update(|open| *open = !*open);
        }
        on:keydown=move |ev: web_sys::KeyboardEvent| {
          let key = crate::utils::safe_key(&ev);
          if key == "Enter" || key == " " {
            ev.prevent_default();
            menu_open.update(|open| *open = !*open);
          }
        }
        data-testid="sidebar-conversation-actions-btn"
      >
        <Icon icon=i::LuEllipsis />
      </button>

      <Show when=move || menu_open.get() fallback=|| ()>
        <SidebarConversationMenu
          conversation_id=conv_id_menu.clone()
          pinned=pinned_sig
          muted=muted_sig
          archived=archived_sig
          open=menu_open
          on_delete=Callback::new(move |_: ()| delete_modal_open.set(true))
        />
      </Show>

      // G21 — delete-confirmation modal. Owned by the row so the
      // dialog renders independently of the chat view's
      // `DialogState`. The modal portal-renders to `#modal-root`
      // (via `ModalWrapper::Portal`), so layout is unaffected by
      // the sidebar's `backdrop-filter` containing-block trap.
      {
        let conv_for_delete = conv_id.clone();
        let on_close: Callback<()> = Callback::new(move |_: ()| delete_modal_open.set(false));
        let on_cancel: Callback<()> = Callback::new(move |_: ()| delete_modal_open.set(false));
        let on_confirm: Callback<()> = Callback::new(move |_: ()| {
          app_state.purge_conversation(&conv_for_delete);
          delete_modal_open.set(false);
        });
        view! {
          <ModalWrapper
            on_close=on_close
            open=delete_modal_open_signal
            size=ModalSize::Small
            class="dialog-box"
            labelled_by="sidebar-delete-modal-title"
            testid="sidebar-delete-modal"
          >
            <div class="modal-body">
              <h2
                id="sidebar-delete-modal-title"
                class="dialog-message"
                data-testid="sidebar-delete-modal-title"
              >
                {move || t_string!(i18n, sidebar.delete_confirm_title)}
              </h2>
              <p
                class="dialog-message"
                data-testid="sidebar-delete-modal-body"
              >
                {move || t_string!(i18n, sidebar.delete_confirm_body)}
              </p>
              <div class="dialog-actions">
                <button
                  type="button"
                  class="dialog-btn dialog-btn-cancel"
                  aria-label=move || t_string!(i18n, sidebar.delete_confirm_cancel)
                  title=move || t_string!(i18n, sidebar.delete_confirm_cancel)
                  on:click=move |_| on_cancel.run(())
                  data-testid="sidebar-delete-modal-cancel"
                >
                  <Icon icon=i::LuX />
                </button>
                <button
                  type="button"
                  class="dialog-btn dialog-btn-ok dialog-btn-danger"
                  aria-label=move || t_string!(i18n, sidebar.delete_confirm_ok)
                  title=move || t_string!(i18n, sidebar.delete_confirm_ok)
                  on:click=move |_| on_confirm.run(())
                  data-testid="sidebar-delete-modal-confirm"
                >
                  <Icon icon=i::LuTrash2 />
                </button>
              </div>
            </div>
          </ModalWrapper>
        }
      }
    </div>
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn arrow_down_advances_with_wrap() {
    assert_eq!(next_focus_index("ArrowDown", 0, 3), Some(1));
    assert_eq!(next_focus_index("ArrowDown", 1, 3), Some(2));
    // Wrap around at the end of the list.
    assert_eq!(next_focus_index("ArrowDown", 2, 3), Some(0));
  }

  #[test]
  fn arrow_up_retreats_with_wrap() {
    assert_eq!(next_focus_index("ArrowUp", 1, 3), Some(0));
    // Wrap around at the start of the list.
    assert_eq!(next_focus_index("ArrowUp", 0, 3), Some(2));
  }

  #[test]
  fn home_and_end_jump_to_extremes() {
    assert_eq!(next_focus_index("Home", 5, 10), Some(0));
    assert_eq!(next_focus_index("End", 0, 10), Some(9));
  }

  #[test]
  fn unsupported_keys_return_none() {
    assert_eq!(next_focus_index("Tab", 0, 3), None);
    assert_eq!(next_focus_index("a", 0, 3), None);
  }

  #[test]
  fn empty_list_returns_none_for_every_key() {
    for key in ["ArrowUp", "ArrowDown", "Home", "End"] {
      assert_eq!(next_focus_index(key, 0, 0), None);
    }
  }
}
