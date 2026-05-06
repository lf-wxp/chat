//! Shared CSS-class helpers for settings form controls.
//!
//! Every section file in this module uses inline segmented / toggle
//! markup — the project rule "one component per file" means we do
//! not expose wrapper components. Instead, these small helper
//! functions keep the class-picking logic in one place so visual
//! state stays consistent across every row.

/// Pick the CSS class for a segmented-control item.
///
/// `active` corresponds to the visually-selected state. Returns a
/// `&'static str` so the classname can be fed directly to a reactive
/// closure without allocation.
#[must_use]
pub fn segmented_item_class(active: bool) -> &'static str {
  if active {
    "segmented-item is-active"
  } else {
    "segmented-item"
  }
}

/// Pick the CSS class for a toggle (switch) root element.
#[must_use]
pub fn toggle_root_class(on: bool) -> &'static str {
  if on {
    "settings-toggle is-on"
  } else {
    "settings-toggle"
  }
}
