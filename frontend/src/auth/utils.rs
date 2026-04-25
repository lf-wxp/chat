//! Auth utility helpers.

use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

/// Extract a user-friendly message from a `JsValue` error.
///
/// Attempts to read `.message` from the JS error object. Falls back to
/// a plain string if possible and finally to `JSON.stringify` so that
/// opaque JS objects still produce useful trace information instead of
/// a generic "Unknown error" (Issue-6 fix; R2-Issue-8 hardening).
pub(crate) fn format_js_error(e: &JsValue) -> String {
  // Explicit null / undefined branches so logs do not degrade to
  // "Unknown error" when a fetch() rejects with `undefined` (happens in
  // Firefox when the page is navigated away mid-request) (P2-4 fix).
  if e.is_null() {
    return "null".to_string();
  }
  if e.is_undefined() {
    return "undefined".to_string();
  }
  // Try DOMException first (fetch failures, abort errors, network errors, etc.)
  if let Some(dom_err) = e.dyn_ref::<web_sys::DomException>() {
    return dom_err.message();
  }
  // Try Error.prototype.message (TypeError, RangeError, etc.)
  if let Some(err) = e.dyn_ref::<js_sys::Error>() {
    return err.message().into();
  }
  // Fall back to stringifying the value directly.
  if let Some(s) = e.as_string() {
    return s;
  }
  // Last resort: run JSON.stringify so non-Error JS objects still reveal
  // their shape in logs instead of degrading to "Unknown error".
  if let Ok(json) = js_sys::JSON::stringify(e)
    && let Some(s) = json.as_string()
    && !s.is_empty()
    && s != "{}"
  {
    return s;
  }
  "Unknown error".to_string()
}
