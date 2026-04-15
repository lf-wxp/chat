//! Common utility functions.
//!
//! Shared helpers for localStorage access and other browser APIs.

/// Read a value from localStorage by key.
///
/// Returns `None` if the window, storage, or key is unavailable.
#[must_use]
pub fn load_from_local_storage(key: &str) -> Option<String> {
  web_sys::window()
    .and_then(|w| w.local_storage().ok())
    .flatten()
    .and_then(|s| s.get_item(key).ok())
    .flatten()
}

/// Write a value to localStorage.
///
/// Silently ignores failures (e.g., storage quota exceeded or no window).
pub fn save_to_local_storage(key: &str, value: &str) {
  if let Some(window) = web_sys::window()
    && let Ok(Some(storage)) = window.local_storage()
  {
    let _ = storage.set_item(key, value);
  }
}
