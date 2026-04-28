//! Image thumbnail generation for file transfers (Req 6.7).
//!
//! Generates a 128×128 thumbnail from an outgoing image file using
//! the `<canvas>` API (WASM) or a native stub (test). The thumbnail
//! is stored as a separate blob URL so the `<img>` element in the
//! file card loads a small preview instead of decoding the full
//! original image.
//!
//! P2-E fix: uses `canvas.to_blob()` + `URL.createObjectURL()` to
//! produce a true `blob:` URL rather than a ~33 %-larger Base64
//! Data URL, and exposes [`revoke_thumbnail_url`] so callers can
//! release the blob when the transfer is cancelled or dismissed.

/// Maximum thumbnail dimension in pixels.
pub const THUMBNAIL_SIZE: u32 = 128;

/// Generate a thumbnail blob URL from a full-size image blob URL.
///
/// On WASM this creates an offscreen `<canvas>`, draws the image
/// scaled down to `THUMBNAIL_SIZE × THUMBNAIL_SIZE`, and exports
/// the result as a new blob URL via `canvas.to_blob()` +
/// `URL.createObjectURL()`. On native it returns `None` (tests don't
/// need thumbnails).
#[cfg(target_arch = "wasm32")]
pub async fn generate_thumbnail_url(object_url: &str) -> Option<String> {
  use js_sys::Function;
  use wasm_bindgen::JsCast;
  use wasm_bindgen::closure::Closure;
  use web_sys::{Blob, HtmlCanvasElement, HtmlImageElement, Url};

  let img = HtmlImageElement::new().ok()?;
  img.set_cross_origin(Some("anonymous"));

  // Wait for the image to load.
  let promise = js_sys::Promise::new(&mut |resolve, reject| {
    img.set_onload(Some(&resolve));
    img.set_onerror(Some(&reject));
  });
  img.set_src(object_url);
  let _ = wasm_bindgen_futures::JsFuture::from(promise).await.ok()?;

  let nat_w = img.natural_width();
  let nat_h = img.natural_height();
  if nat_w == 0 || nat_h == 0 {
    return None;
  }

  // Fit into THUMBNAIL_SIZE while preserving aspect ratio.
  let scale = f64::from(THUMBNAIL_SIZE) / f64::from(nat_w.max(nat_h));
  #[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
  )]
  let thumb_w = (f64::from(nat_w) * scale).round() as u32;
  #[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
  )]
  let thumb_h = (f64::from(nat_h) * scale).round() as u32;

  let window = web_sys::window()?;
  let document = window.document()?;
  let canvas: HtmlCanvasElement = document.create_element("canvas").ok()?.dyn_into().ok()?;
  canvas.set_width(thumb_w);
  canvas.set_height(thumb_h);

  let ctx = canvas
    .get_context("2d")
    .ok()??
    .dyn_into::<web_sys::CanvasRenderingContext2d>()
    .ok()?;
  ctx
    .draw_image_with_html_image_element_and_dw_and_dh(
      &img,
      0.0,
      0.0,
      f64::from(thumb_w),
      f64::from(thumb_h),
    )
    .ok()?;

  // Export as a real Blob and wrap it in an object URL. `to_blob`
  // is callback-based, so we adapt it into a Promise.
  let blob_promise = js_sys::Promise::new(&mut |resolve, _reject| {
    let resolve_fn: Function = resolve;
    let cb = Closure::once_into_js(move |blob: wasm_bindgen::JsValue| {
      let _ = resolve_fn.call1(&wasm_bindgen::JsValue::NULL, &blob);
    });
    let _ = canvas.to_blob(cb.as_ref().unchecked_ref());
  });
  let blob_value = wasm_bindgen_futures::JsFuture::from(blob_promise)
    .await
    .ok()?;
  let blob: Blob = blob_value.dyn_into().ok()?;
  Url::create_object_url_with_blob(&blob).ok()
}

/// Native stub — always returns `None` (no browser canvas available).
#[cfg(not(target_arch = "wasm32"))]
pub async fn generate_thumbnail_url(_object_url: &str) -> Option<String> {
  None
}

/// Release the blob URL produced by [`generate_thumbnail_url`].
///
/// Safe to call with any string: non-`blob:` URLs are no-ops on
/// modern browsers and errors are silently ignored.
#[cfg(target_arch = "wasm32")]
pub fn revoke_thumbnail_url(url: &str) {
  if !url.starts_with("blob:") {
    return;
  }
  let _ = web_sys::Url::revoke_object_url(url);
}

/// Native stub.
#[cfg(not(target_arch = "wasm32"))]
pub fn revoke_thumbnail_url(_url: &str) {}
