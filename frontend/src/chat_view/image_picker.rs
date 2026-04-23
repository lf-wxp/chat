//! Image picker overlay.
//!
//! Renders a hidden `<input type="file">` triggered by the toolbar
//! button and performs a best-effort pipeline:
//!
//! 1. User selects a file via the native picker.
//! 2. A `FileReader` reads the image into an ArrayBuffer.
//! 3. `Url.createObjectURL` produces an object URL for the full image
//!    and (for the MVP) the same URL is reused as the thumbnail URL.
//! 4. The resulting `ImagePayload` is handed to `ChatManager::send_image`.
//!
//! Real thumbnail generation (scaling to <=256 px) is left as a future
//! enhancement — plumbing it through `<canvas>` + `toBlob` would pull
//! in extra web-sys features. For correctness today, the thumbnail URL
//! simply points at the full-resolution image so the UI still renders.
//! The wire-level `thumbnail` bytes we ship are the raw image bytes
//! (callers can crop/downsample later without a protocol change).

use crate::chat::manager::ImagePayload;
use crate::chat::use_chat_manager;
use crate::i18n;
use crate::state::ConversationId;
use leptos::prelude::*;
use leptos_i18n::t_string;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{File, FileReader, HtmlInputElement, Url};

/// Image picker overlay (thin button + hidden file input).
#[component]
pub fn ImagePicker(
  /// Active conversation.
  conv: Signal<Option<ConversationId>>,
  /// Visibility signal (the picker opens the native dialog each time
  /// this becomes `true`, then resets itself to `false`).
  visible: RwSignal<bool>,
) -> impl IntoView {
  let manager = use_chat_manager();
  let i18n = i18n::use_i18n();

  // Trigger the native picker whenever `visible` flips to `true`.
  Effect::new(move |_| {
    if visible.get() {
      if let Some(el) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("chat-image-picker-input"))
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
      {
        el.click();
      }
      visible.set(false);
    }
  });

  let on_change = {
    let manager = manager.clone();
    move |ev: leptos::ev::Event| {
      let Some(target) = ev.target() else { return };
      let Ok(input) = target.dyn_into::<HtmlInputElement>() else {
        return;
      };
      let Some(files) = input.files() else { return };
      if files.length() == 0 {
        return;
      }
      let Some(file) = files.item(0) else { return };
      let Some(conv_id) = conv.get_untracked() else {
        return;
      };
      if let Err(err) = send_file(&manager, conv_id, file) {
        web_sys::console::warn_1(&format!("[chat] image picker failed: {err:?}").into());
      }
      // Reset so picking the same file twice fires `change`.
      input.set_value("");
    }
  };

  view! {
    <input
      id="chat-image-picker-input"
      type="file"
      accept="image/*"
      style="display:none"
      aria-label=move || t_string!(i18n, chat.attach_image)
      on:change=on_change
      data-testid="image-picker-input"
    />
  }
}

/// Read a `File`, create object URLs, resolve image dimensions via an
/// `HtmlImageElement`, and dispatch `send_image`.
fn send_file(
  manager: &crate::chat::ChatManager,
  conv: ConversationId,
  file: File,
) -> Result<(), wasm_bindgen::JsValue> {
  let url = Url::create_object_url_with_blob(&file)?;
  let thumbnail_url = url.clone();

  // Read the full bytes via `FileReader` so we can ship them on the
  // wire. The reader is async; we retain the closure through an `Rc`
  // so it survives until the `loadend` event fires.
  let reader = FileReader::new()?;
  let reader_clone = reader.clone();
  let manager_clone = manager.clone();
  let url_for_img = url.clone();

  let on_load = Closure::once_into_js(move || {
    let Ok(buffer) = reader_clone.result() else {
      return;
    };
    let bytes = js_sys::Uint8Array::new(&buffer).to_vec();

    // Use HtmlImageElement to obtain actual width/height before sending.
    let Ok(img) = web_sys::HtmlImageElement::new() else {
      // Fallback: send with zero dimensions if element creation fails.
      let payload = ImagePayload {
        image_data: bytes.clone(),
        thumbnail: bytes,
        width: 0,
        height: 0,
        object_url: url_for_img,
        thumbnail_url,
      };
      let _ = manager_clone.send_image(conv, payload);
      return;
    };

    let bytes_for_cb = bytes.clone();
    let url_for_cb = url_for_img.clone();
    let thumb_for_cb = thumbnail_url.clone();

    let on_img_load = Closure::once_into_js(move || {
      let w = img.natural_width();
      let h = img.natural_height();
      let payload = ImagePayload {
        image_data: bytes_for_cb.clone(),
        thumbnail: bytes_for_cb,
        width: w,
        height: h,
        object_url: url_for_cb,
        thumbnail_url: thumb_for_cb,
      };
      let _ = manager_clone.send_image(conv, payload);
    });

    let img2 = web_sys::HtmlImageElement::new().unwrap();
    img2.set_onload(Some(on_img_load.unchecked_ref()));
    img2.set_src(&url_for_img);
  });

  reader.set_onloadend(Some(on_load.unchecked_ref()));
  reader.read_as_array_buffer(&file)?;
  Ok(())
}
