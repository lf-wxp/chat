//! Drag-and-drop overlay component

use leptos::prelude::*;
use leptos_i18n::t;

use crate::i18n::*;

/// Drag-and-drop file upload overlay
#[component]
pub fn DragOverlay(
  /// Whether files are being dragged over the chat area
  is_dragging: RwSignal<bool>,
) -> impl IntoView {
  let i18n = use_i18n();

  move || {
    if !is_dragging.get() {
      return view! { <div class="drop-overlay-hidden"></div> }.into_any();
    }
    view! {
      <div class="drop-overlay">
        <div class="drop-overlay-content">
          <div class="drop-overlay-icon">"📂"</div>
          <div class="drop-overlay-title">{t!(i18n, chat_drop_to_send)}</div>
          <div class="drop-overlay-hint">{t!(i18n, chat_drop_supported_files)}</div>
        </div>
      </div>
    }.into_any()
  }
}
