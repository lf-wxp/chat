//! Generic anchored dropdown menu.
//!
//! Renders a fixed-position popover next to a trigger button with:
//!
//! * Viewport-aware placement — opens below the trigger by default,
//!   flips above when there isn't enough room, and clamps horizontally
//!   so the panel never bleeds off the screen edge.
//! * Escape-to-close and outside-pointerdown-to-close listeners.
//! * A spring entrance animation; the direction is inverted when the
//!   panel opens upward so it always appears to "grow out of" the
//!   trigger.
//!
//! The panel is `position: fixed`, so it escapes any `overflow: hidden`
//! / `backdrop-filter` containing blocks on ancestor elements (this
//! is why the sidebar conversation menu previously got clipped by
//! `.sidebar-conversation`'s `overflow: hidden`).
//!
//! # Example
//!
//! ```ignore
//! let open = RwSignal::new(false);
//! let trigger = NodeRef::<html::Button>::new();
//!
//! view! {
//!   <button node_ref=trigger on:click=move |_| open.update(|v| *v = !*v)>
//!     "⋯"
//!   </button>
//!   <Show when=move || open.get()>
//!     <DropdownMenu open=open trigger=trigger>
//!       <DropdownMenuItem on_click=Callback::new(|_| /* … */)>
//!         <Icon icon=i::LuPin /> "Pin"
//!       </DropdownMenuItem>
//!     </DropdownMenu>
//!   </Show>
//! }
//! ```

use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use leptos_use::use_event_listener;
use wasm_bindgen::JsCast;
use web_sys::Element;

/// Estimated height of one menu row (padding + line-height) used by
/// the placement heuristic before the panel has been measured.
const ITEM_HEIGHT_PX: f64 = 40.0;
/// Vertical gap between the trigger and the popover.
const GAP_PX: f64 = 4.0;
/// Minimum distance to keep from the viewport edges.
const EDGE_PX: f64 = 8.0;
/// Default minimum width in px (matches the CSS `--dropdown-min-width`
/// custom property's `11.5rem` @ 16px base).
const MIN_WIDTH_PX: f64 = 184.0;

/// Generic anchored dropdown menu.
///
/// The parent controls visibility via the `open` signal; the menu
/// flips it back to `false` when the user dismisses the panel.
/// `trigger` must reference the button that toggles the menu — its
/// bounding rect is used to compute the popover's fixed position.
#[component]
pub fn DropdownMenu(
  /// Visibility signal. The menu sets this to `false` on dismiss.
  open: RwSignal<bool>,
  /// Reference to the trigger button for anchor positioning.
  trigger: NodeRef<html::Button>,
  /// Estimated number of items, used to size the flip heuristic.
  /// Defaults to 4. Provide a closer estimate for very tall menus.
  #[prop(default = 4)]
  estimated_items: usize,
  /// Optional additional class(es) applied to the panel root.
  #[prop(optional, into)]
  class: Option<String>,
  /// Optional `data-testid` for the panel root. Empty string = omitted.
  #[prop(optional, into)]
  testid: Signal<String>,
  /// Menu items.
  children: Children,
) -> impl IntoView {
  // Popover position in viewport coordinates. Defaults off-screen so
  // the panel never flashes at (0,0) before the first measurement.
  let pos_left = RwSignal::new(-9999.0_f64);
  let pos_top = RwSignal::new(-9999.0_f64);
  let open_above = RwSignal::new(false);

  // Compute the popover position from the trigger's bounding rect
  // whenever the menu opens.
  Effect::new(move |_| {
    if !open.get() {
      return;
    }
    let Some(btn) = trigger.get() else {
      return;
    };
    // `getBoundingClientRect` lives on `web_sys::Element`, not on
    // the concrete `HtmlButtonElement` — upcast first.
    let Some(el) = btn.dyn_ref::<Element>() else {
      return;
    };
    let rect = el.get_bounding_client_rect();
    let Some(win) = web_sys::window() else { return };
    let vw = win
      .inner_width()
      .ok()
      .and_then(|v| v.as_f64())
      .unwrap_or(1024.0);
    let vh = win
      .inner_height()
      .ok()
      .and_then(|v| v.as_f64())
      .unwrap_or(768.0);

    let menu_height = (estimated_items as f64) * ITEM_HEIGHT_PX + 2.0 * GAP_PX;
    let (left, top, above) = compute_placement(PlacementInput {
      trigger_left: rect.left(),
      trigger_right: rect.right(),
      trigger_top: rect.top(),
      trigger_bottom: rect.bottom(),
      viewport_width: vw,
      viewport_height: vh,
      menu_width: MIN_WIDTH_PX,
      menu_height,
    });
    pos_left.set(left);
    pos_top.set(top);
    open_above.set(above);
  });

  // Close on Escape. `stop_propagation` prevents any global Escape
  // handler from also firing (e.g. a modal underneath would close
  // simultaneously).
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

  // Close on outside pointer-down. Clicks on the menu itself or the
  // trigger button pass through — the trigger is handled by the
  // parent and would otherwise flap the menu open-and-closed.
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
      let inside_menu = target.closest(".dropdown-menu").ok().flatten().is_some();
      let on_trigger = target
        .closest(".dropdown-menu-trigger")
        .ok()
        .flatten()
        .is_some();
      if !inside_menu && !on_trigger {
        open.set(false);
      }
    },
  );

  let panel_class = move || {
    let mut cls = String::from("dropdown-menu");
    if open_above.get() {
      cls.push_str(" dropdown-menu--above");
    }
    if let Some(extra) = &class {
      cls.push(' ');
      cls.push_str(extra);
    }
    cls
  };

  view! {
    <div
      class=panel_class
      role="menu"
      aria-orientation="vertical"
      style=move || format!("left: {}px; top: {}px;", pos_left.get(), pos_top.get())
      on:click=move |ev: ev::MouseEvent| ev.stop_propagation()
      data-testid=move || {
        let s = testid.get();
        if s.is_empty() { None } else { Some(s) }
      }
    >
      {children()}
    </div>
  }
}

/// A single row inside a [`DropdownMenu`].
///
/// Renders a `<button role="menuitem">` that closes the parent menu
/// when clicked. Pass `danger=true` for destructive actions — the row
/// turns red on hover to signal the destructive affordance.
#[component]
pub fn DropdownMenuItem(
  /// Click handler. The parent menu closes itself after dispatch.
  on_click: Callback<()>,
  /// Render as a destructive action (red hover state).
  #[prop(default = false)]
  danger: bool,
  /// Disabled state — the row is greyed out and not clickable.
  #[prop(default = false)]
  disabled: bool,
  /// Accessible label. Falls back to the text content when omitted.
  /// Accepts either a static `String` or a reactive closure.
  #[prop(optional, into)]
  aria_label: Signal<String>,
  /// Optional tooltip. Accepts either a static `String` or a reactive
  /// closure.
  #[prop(optional, into)]
  title: Signal<String>,
  /// Optional `data-testid`.
  #[prop(optional, into)]
  testid: Signal<String>,
  /// Row content — typically an `<Icon>` followed by a `<span>` label.
  children: Children,
) -> impl IntoView {
  let class = if danger {
    "dropdown-menu__item dropdown-menu__item--danger"
  } else {
    "dropdown-menu__item"
  };

  view! {
    <button
      type="button"
      class=class
      role="menuitem"
      aria-label=move || {
        let s = aria_label.get();
        if s.is_empty() { None } else { Some(s) }
      }
      title=move || {
        let s = title.get();
        if s.is_empty() { None } else { Some(s) }
      }
      disabled=disabled
      data-testid=move || {
        let s = testid.get();
        if s.is_empty() { None } else { Some(s) }
      }
      on:click=move |_| on_click.run(())
    >
      {children()}
    </button>
  }
}

/// Inputs for the dropdown placement computation.
#[derive(Debug, Clone, Copy)]
pub struct PlacementInput {
  pub trigger_left: f64,
  pub trigger_right: f64,
  pub trigger_top: f64,
  pub trigger_bottom: f64,
  pub viewport_width: f64,
  pub viewport_height: f64,
  pub menu_width: f64,
  pub menu_height: f64,
}

/// Pure placement helper, exposed for unit tests. Returns
/// `(left, top, open_above)` in viewport coordinates.
#[must_use]
pub fn compute_placement(input: PlacementInput) -> (f64, f64, bool) {
  let PlacementInput {
    trigger_left,
    trigger_right,
    trigger_top,
    trigger_bottom,
    viewport_width,
    viewport_height,
    menu_width,
    menu_height,
  } = input;

  // Horizontal: prefer right-aligned, fall back to left-aligned,
  // then clamp to viewport.
  let mut left = trigger_right - menu_width;
  if left < EDGE_PX {
    left = trigger_left;
  }
  if left + menu_width > viewport_width - EDGE_PX {
    left = (viewport_width - EDGE_PX - menu_width).max(EDGE_PX);
  }

  let space_below = viewport_height - trigger_bottom - GAP_PX - EDGE_PX;
  let space_above = trigger_top - GAP_PX - EDGE_PX;
  let (top, above) = if space_below >= menu_height || space_below >= space_above {
    (trigger_bottom + GAP_PX, false)
  } else {
    (trigger_top - GAP_PX - menu_height, true)
  };

  (left, top, above)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn input(
    trigger_left: f64,
    trigger_right: f64,
    trigger_top: f64,
    trigger_bottom: f64,
    viewport_width: f64,
    viewport_height: f64,
  ) -> PlacementInput {
    PlacementInput {
      trigger_left,
      trigger_right,
      trigger_top,
      trigger_bottom,
      viewport_width,
      viewport_height,
      menu_width: 184.0,
      menu_height: 190.0,
    }
  }

  #[test]
  fn prefers_below_and_right_aligned() {
    let (left, top, above) = compute_placement(input(100.0, 200.0, 100.0, 150.0, 1024.0, 768.0));
    assert!(!above);
    assert_eq!(top, 154.0); // 150 + 4
    assert_eq!(left, 16.0); // 200 - 184
  }

  #[test]
  fn flips_above_when_no_room_below() {
    let (_, top, above) = compute_placement(input(100.0, 200.0, 600.0, 650.0, 1024.0, 700.0));
    assert!(above);
    // top = trigger_top - gap - menu_height = 600 - 4 - 190 = 406
    assert_eq!(top, 406.0);
  }

  #[test]
  fn clamps_to_left_edge() {
    let (left, _, _) = compute_placement(input(2.0, 6.0, 100.0, 150.0, 1024.0, 768.0));
    // Would be 6 - 184 = -178 → falls back to trigger_left = 2
    assert_eq!(left, 2.0);
  }

  #[test]
  fn clamps_to_right_edge() {
    let (left, _, _) = compute_placement(input(1000.0, 1016.0, 100.0, 150.0, 1024.0, 768.0));
    // 1016 - 184 = 832, fits in 1024-8=1016, so left = 832
    assert_eq!(left, 832.0);
  }
}
