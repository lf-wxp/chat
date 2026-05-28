//! Debug Panel component.
//!
//! Provides a slide-out debug log viewer accessible via `Ctrl/Cmd + Shift + D`.
//! Features:
//! - Smooth enter/exit CSS transition animations
//! - Scrollable, filterable log viewer
//! - Filter by log level and module
//! - Full-text search
//! - Export logs as JSON
//! - Clear buffer

use super::debug_log_entry::DebugLogEntry;
use crate::logging::{LogLevel, LoggerState};
use crate::state::AppState;
use leptos::prelude::*;

/// Reactive handle to the Debug Panel's visibility signal, published
/// via Leptos context so unrelated components (Settings drawer, etc.)
/// can open the panel without knowing about its internals.
#[derive(Debug, Clone, Copy)]
pub struct DebugPanelVisibility(pub RwSignal<bool>);

/// Install [`DebugPanelVisibility`] at the root of the Leptos tree
/// **before** any sibling component that might consume it (e.g. the
/// Settings drawer's "Open Debug Panel" button). Providing the
/// context inside [`DebugPanel`] itself is not sufficient, because
/// `provide_context` is only visible to descendants — sibling
/// components would get `None` and the button would become a silent
#[must_use]
pub fn provide_debug_panel_visibility() -> DebugPanelVisibility {
  let visible = RwSignal::new(false);
  let handle = DebugPanelVisibility(visible);
  provide_context(handle);
  handle
}

/// Debug Panel component.
///
/// Renders a slide-out panel on the right side of the screen.
/// The panel is always present in the DOM but hidden via CSS.
/// Toggled via `Ctrl/Cmd + Shift + D` keyboard shortcut.
/// Uses CSS transitions for smooth enter/exit animations.
#[component]
pub fn DebugPanel() -> impl IntoView {
  let app_state = expect_context::<AppState>();
  let logger = expect_context::<LoggerState>();

  // Pull the visibility signal from the ancestor-provided context.
  // `provide_debug_panel_visibility` must have been called by the
  // root `App` component before `DebugPanel` is mounted.
  let DebugPanelVisibility(visible) = expect_context::<DebugPanelVisibility>();

  // Filter state
  let search_text = RwSignal::new(String::new());
  let level_filter = RwSignal::new(LogLevel::Trace); // Show all by default
  let module_filter_text = RwSignal::new(String::new());

  // Register global keyboard shortcut (Ctrl/Cmd + Shift + D)
  register_keyboard_shortcut(visible);

  // Derived: filtered log entries
  let filtered_entries = Signal::derive(move || {
    let buffer = logger.buffer.get();
    let entries = buffer.entries();
    let search = search_text.get().to_lowercase();
    let max_level = level_filter.get();
    let module_text = module_filter_text.get();

    entries
      .into_iter()
      .filter(|e| e.level <= max_level)
      .filter(|e| {
        if module_text.is_empty() {
          true
        } else {
          module_text
            .split(',')
            .any(|seg| e.module.contains(seg.trim()))
        }
      })
      .filter(|e| {
        if search.is_empty() {
          true
        } else {
          e.message.to_lowercase().contains(&search) || e.module.to_lowercase().contains(&search)
        }
      })
      .collect::<Vec<_>>()
  });

  // Entry count for footer
  let total_count = Signal::derive(move || logger.buffer.get().len());

  // Handle export JSON
  let handle_export = move |_| {
    crate::logging::download_diagnostic_report(&logger, &app_state);
  };

  // Handle clear buffer
  let handle_clear = move |_| {
    logger.buffer.update(|buf| buf.clear());
  };

  // Handle close
  let handle_close = move |_| {
    visible.set(false);
  };

  // Level filter buttons
  let levels = [
    (LogLevel::Error, "ERR"),
    (LogLevel::Warn, "WARN"),
    (LogLevel::Info, "INFO"),
    (LogLevel::Debug, "DBG"),
    (LogLevel::Trace, "TRC"),
  ];

  // Dynamic class: always in DOM, toggle visibility via CSS class
  let backdrop_class = move || {
    if visible.get() {
      "debug-panel-backdrop debug-panel-visible"
    } else {
      "debug-panel-backdrop"
    }
  };

  view! {
    <div
      class=backdrop_class
      on:click=move |ev| {
        // Close when clicking backdrop (not panel itself)
        let target = ev.target().unwrap();
        let current = ev.current_target().unwrap();
        if target == current {
          visible.set(false);
        }
      }
    >
      <div class="debug-panel" role="dialog" aria-label="Debug Panel">
        // Header
        <div class="debug-panel-header">
          <span class="debug-panel-title">"Debug Panel"</span>
          <div class="debug-panel-actions">
            <button
              class="btn-ghost btn-sm"
              aria-label="Export diagnostic report"
              on:click=handle_export
            >
              "Export"
            </button>
            <button
              class="btn-ghost btn-sm"
              aria-label="Clear log buffer"
              on:click=handle_clear
            >
              "Clear"
            </button>
            <button
              class="btn-ghost btn-sm"
              aria-label="Close debug panel"
              on:click=handle_close
            >
              "\u{2715}"
            </button>
          </div>
        </div>

        // Toolbar
        <div class="debug-panel-toolbar">
          <input
            type="search"
            class="debug-panel-search"
            placeholder="Search logs..."
            aria-label="Search logs"
            prop:value=move || search_text.get()
            on:input=move |ev| {
              search_text.set(event_target_value(&ev));
            }
          />
          <div class="debug-level-filter">
            {levels
              .into_iter()
              .map(|(level, label)| {
                let is_active = Signal::derive(move || level_filter.get() >= level);
                view! {
                  <button
                    class=move || {
                      if is_active.get() {
                        "debug-level-btn debug-level-active"
                      } else {
                        "debug-level-btn"
                      }
                    }
                    on:click=move |_| {
                      level_filter.set(level);
                    }
                    aria-label=format!("Filter: {}", label)
                  >
                    {label}
                  </button>
                }
              })
              .collect::<Vec<_>>()}
          </div>
          <input
            type="text"
            class="debug-panel-search"
            placeholder="Module filter (comma-separated)"
            aria-label="Module filter"
            style="max-width: 14rem;"
            prop:value=move || module_filter_text.get()
            on:input=move |ev| {
              module_filter_text.set(event_target_value(&ev));
            }
          />
        </div>

        // Log list
        <div class="debug-log-list" aria-live="polite">
          <For
            each=move || filtered_entries.get()
            key=|entry| (entry.timestamp, entry.module.clone(), entry.message.clone())
            children=move |entry| {
              view! { <DebugLogEntry entry=entry /> }
            }
          />
        </div>

        // Footer
        <div class="debug-panel-footer">
          <span>
            {move || {
              let filtered = filtered_entries.get().len();
              let total = total_count.get();
              format!("{} / {} entries", filtered, total)
            }}
          </span>
          <span>"Ctrl+Shift+D to toggle"</span>
        </div>
      </div>
    </div>
  }
}

/// Register the global keyboard shortcut for toggling the debug panel.
///
/// Uses `leptos_use::use_event_listener` so the listener is
/// automatically cleaned up when the component unmounts — no manual
/// `Closure` / `StoredValue` / `remove_event_listener` bookkeeping.
fn register_keyboard_shortcut(visible: RwSignal<bool>) {
  use leptos::ev::keydown;
  use leptos_use::use_event_listener;

  let _cleanup = use_event_listener(window(), keydown, move |ev: web_sys::KeyboardEvent| {
    // Ctrl+Shift+D (Windows/Linux) or Cmd+Shift+D (macOS)
    let is_mod = ev.ctrl_key() || ev.meta_key();
    if is_mod && ev.shift_key() && crate::utils::safe_key(&ev) == "D" {
      ev.prevent_default();
      visible.update(|v| *v = !*v);
    }
  });
}
