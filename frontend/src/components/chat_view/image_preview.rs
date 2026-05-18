//! Full-screen image preview overlay.
//!
//! Opened when the user clicks an image bubble. Dismissed by any
//! keyboard or pointer interaction. Uses a global signal exposed via
//! the parent `ChatView` so any bubble can trigger the overlay.

use leptos::ev;
use leptos::prelude::*;
use leptos_use::use_event_listener;

/// Preview overlay wrapping a single full-size image URL.
///
/// The `url` signal carries `None` while the overlay is dismissed.
#[component]
pub fn ImagePreviewOverlay(url: RwSignal<Option<String>>) -> impl IntoView {
  // Window-level Escape listener — the overlay div is not focusable
  // (it carries neither `tabindex` nor a focusable child), so a
  // `keydown` handler attached directly to it would never fire. The
  // shared pattern across `ModalWrapper`, `SidebarConversationMenu`
  // and `StickerPanel` is to attach to `window` and gate on the
  // visibility signal. This makes both keyboard users AND the E2E
  // suite (`page.locator(...).press("Escape")`) able to dismiss the
  // overlay (G24).
  let _ = use_event_listener(
    leptos_use::use_window(),
    ev::keydown,
    move |ev: web_sys::KeyboardEvent| {
      if url.get_untracked().is_none() {
        return;
      }
      if crate::utils::safe_key(&ev) == "Escape" {
        url.set(None);
        ev.stop_propagation();
      }
    },
  );

  view! {
    <Show when=move || url.get().is_some() fallback=|| ()>
      {move || {
        let Some(u) = url.get() else { return ().into_any() };
        view! {
          <div
            class="image-preview-overlay"
            role="dialog"
            aria-modal="true"
            tabindex="-1"
            data-testid="image-preview"
            on:click=move |_| url.set(None)
          >
            <img src=u alt="" />
          </div>
        }.into_any()
      }}
    </Show>
  }
}
