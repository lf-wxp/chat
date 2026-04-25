//! Full-screen image preview overlay.
//!
//! Opened when the user clicks an image bubble. Dismissed by any
//! keyboard or pointer interaction. Uses a global signal exposed via
//! the parent `ChatView` so any bubble can trigger the overlay.

use leptos::prelude::*;

/// Preview overlay wrapping a single full-size image URL.
///
/// The `url` signal carries `None` while the overlay is dismissed.
#[component]
pub fn ImagePreviewOverlay(url: RwSignal<Option<String>>) -> impl IntoView {
  view! {
    <Show when=move || url.get().is_some() fallback=|| ()>
      {move || {
        let Some(u) = url.get() else { return ().into_any() };
        view! {
          <div
            class="image-preview-overlay"
            role="dialog"
            aria-modal="true"
            data-testid="image-preview"
            on:click=move |_| url.set(None)
            on:keydown=move |ev: web_sys::KeyboardEvent| {
              if ev.key() == "Escape" {
                url.set(None);
              }
            }
          >
            <img src=u alt="" />
          </div>
        }.into_any()
      }}
    </Show>
  }
}
