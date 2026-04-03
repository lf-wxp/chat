//! Utility functions

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

/// Get WebSocket server address
///
/// Automatically selects ws:// or wss:// based on the current page protocol
pub fn get_ws_url() -> String {
  if let Some(window) = web_sys::window()
    && let Ok(location) = window.location().href()
  {
    let protocol = if location.starts_with("https") {
      "wss"
    } else {
      "ws"
    };
    let host = window
      .location()
      .host()
      .unwrap_or_else(|_| "localhost:3000".into());
    return format!("{protocol}://{host}/ws");
  }
  "ws://localhost:3000/ws".to_string()
}

/// Format timestamp to human-readable string
pub fn format_timestamp(ts: i64) -> String {
  let now = chrono::Utc::now().timestamp();
  let diff = now - ts;

  if diff < 60 {
    "just now".to_string()
  } else if diff < 3600 {
    format!("{} minutes ago", diff / 60)
  } else if diff < 86400 {
    format!("{} hours ago", diff / 3600)
  } else if diff < 604_800 {
    format!("{} days ago", diff / 86400)
  } else {
    // Use chrono for formatting
    chrono::DateTime::from_timestamp(ts, 0)
      .map(|dt| dt.format("%m-%d %H:%M").to_string())
      .unwrap_or_default()
  }
}

/// Format file size to human-readable string
pub fn format_file_size(bytes: u64) -> String {
  const KB: u64 = 1024;
  const MB: u64 = 1024 * KB;
  const GB: u64 = 1024 * MB;

  if bytes < KB {
    format!("{bytes} B")
  } else if bytes < MB {
    format!("{:.1} KB", bytes as f64 / KB as f64)
  } else if bytes < GB {
    format!("{:.1} MB", bytes as f64 / MB as f64)
  } else {
    format!("{:.2} GB", bytes as f64 / GB as f64)
  }
}

/// Set timeout (setTimeout wrapper)
pub fn set_timeout(callback: impl Fn() + 'static, delay_ms: i32) -> i32 {
  let cb = Closure::<dyn Fn()>::new(callback);
  let id = web_sys::window().map_or(0, |w| {
    w.set_timeout_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), delay_ms)
      .unwrap_or(0)
  });
  cb.forget();
  id
}

/// Clear timeout
pub fn clear_timeout(id: i32) {
  if let Some(window) = web_sys::window() {
    window.clear_timeout_with_handle(id);
  }
}

/// Set interval timer (setInterval wrapper)
pub fn set_interval(callback: impl Fn() + 'static, interval_ms: i32) -> i32 {
  let cb = Closure::<dyn Fn()>::new(callback);
  let id = web_sys::window().map_or(0, |w| {
    w.set_interval_with_callback_and_timeout_and_arguments_0(
      cb.as_ref().unchecked_ref(),
      interval_ms,
    )
    .unwrap_or(0)
  });
  cb.forget();
  id
}

/// Clear interval timer
pub fn clear_interval(id: i32) {
  if let Some(window) = web_sys::window() {
    window.clear_interval_with_handle(id);
  }
}

/// Read value from localStorage
pub fn local_storage_get(key: &str) -> Option<String> {
  web_sys::window()
    .and_then(|w| w.local_storage().ok().flatten())
    .and_then(|s| s.get_item(key).ok().flatten())
}

/// Write value to localStorage
pub fn local_storage_set(key: &str, value: &str) {
  if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
    let _ = storage.set_item(key, value);
  }
}

/// Remove value from localStorage
pub fn local_storage_remove(key: &str) {
  if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
    let _ = storage.remove_item(key);
  }
}

/// Request browser notification permission
pub fn request_notification_permission() {
  if let Some(window) = web_sys::window()
    && let Ok(notification) = js_sys::Reflect::get(&window, &"Notification".into())
    && !notification.is_undefined()
  {
    let _ = js_sys::Reflect::get(&notification, &"requestPermission".into()).and_then(|func| {
      func
        .dyn_ref::<js_sys::Function>()
        .map_or(Ok(JsValue::UNDEFINED), |f| f.call0(&notification))
    });
  }
}

/// Send browser notification
pub fn send_notification(title: &str, body: &str) {
  if let Some(_window) = web_sys::window() {
    let options = web_sys::NotificationOptions::new();
    options.set_body(body);
    let _ = web_sys::Notification::new_with_options(title, &options);
  }
}

/// Get user media stream (camera/microphone)
pub async fn get_user_media(audio: bool, video: bool) -> Result<web_sys::MediaStream, String> {
  let window = web_sys::window().ok_or("Cannot get window")?;
  let navigator = window.navigator();
  let media_devices = navigator
    .media_devices()
    .map_err(|_| "Cannot get MediaDevices")?;

  let constraints = web_sys::MediaStreamConstraints::new();
  constraints.set_audio(&JsValue::from_bool(audio));
  constraints.set_video(&JsValue::from_bool(video));

  let promise = media_devices
    .get_user_media_with_constraints(&constraints)
    .map_err(|e| format!("getUserMedia failed: {e:?}"))?;

  let stream = wasm_bindgen_futures::JsFuture::from(promise)
    .await
    .map_err(|e| format!("Failed to get media stream: {e:?}"))?;

  stream
    .dyn_into::<web_sys::MediaStream>()
    .map_err(|_| "Cannot convert to MediaStream".to_string())
}

/// Get screen sharing media stream (`getDisplayMedia`)
pub async fn get_display_media() -> Result<web_sys::MediaStream, String> {
  let window = web_sys::window().ok_or("Cannot get window")?;
  let navigator = window.navigator();
  let media_devices = navigator
    .media_devices()
    .map_err(|_| "Cannot get MediaDevices")?;

  let constraints = web_sys::DisplayMediaStreamConstraints::new();
  constraints.set_video(&JsValue::from_bool(true));
  constraints.set_audio(&JsValue::from_bool(false));

  let promise = media_devices
    .get_display_media_with_constraints(&constraints)
    .map_err(|e| format!("getDisplayMedia failed: {e:?}"))?;

  let stream = wasm_bindgen_futures::JsFuture::from(promise)
    .await
    .map_err(|e| format!("Failed to get screen sharing stream: {e:?}"))?;

  stream
    .dyn_into::<web_sys::MediaStream>()
    .map_err(|_| "Cannot convert to MediaStream".to_string())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
  use super::*;
  use wasm_bindgen_test::wasm_bindgen_test;

  // =========================================================================
  // format_file_size tests
  // =========================================================================

  #[wasm_bindgen_test]
  fn test_format_file_size_bytes() {
    assert_eq!(format_file_size(0), "0 B");
    assert_eq!(format_file_size(1), "1 B");
    assert_eq!(format_file_size(512), "512 B");
    assert_eq!(format_file_size(1023), "1023 B");
  }

  #[wasm_bindgen_test]
  fn test_format_file_size_kb() {
    assert_eq!(format_file_size(1024), "1.0 KB");
    assert_eq!(format_file_size(1536), "1.5 KB");
    assert_eq!(format_file_size(10240), "10.0 KB");
  }

  #[wasm_bindgen_test]
  fn test_format_file_size_mb() {
    assert_eq!(format_file_size(1024 * 1024), "1.0 MB");
    assert_eq!(format_file_size(5 * 1024 * 1024), "5.0 MB");
    assert_eq!(format_file_size(1024 * 1024 + 512 * 1024), "1.5 MB");
  }

  #[wasm_bindgen_test]
  fn test_format_file_size_gb() {
    assert_eq!(format_file_size(1024 * 1024 * 1024), "1.00 GB");
    assert_eq!(format_file_size(2 * 1024 * 1024 * 1024), "2.00 GB");
  }

  #[wasm_bindgen_test]
  fn test_format_file_size_boundary() {
    // Exactly 1 KB
    assert_eq!(format_file_size(1024), "1.0 KB");
    // Exactly 1 MB
    assert_eq!(format_file_size(1024 * 1024), "1.0 MB");
    // Exactly 1 GB
    assert_eq!(format_file_size(1024 * 1024 * 1024), "1.00 GB");
  }

  // =========================================================================
  // format_timestamp tests (boundary behavior only)
  // =========================================================================

  #[wasm_bindgen_test]
  fn test_format_timestamp_just_now() {
    let now = chrono::Utc::now().timestamp();
    assert_eq!(format_timestamp(now), "just now");
  }

  #[wasm_bindgen_test]
  fn test_format_timestamp_minutes_ago() {
    let ts = chrono::Utc::now().timestamp() - 120; // 2 minutes ago
    let result = format_timestamp(ts);
    assert!(result.contains("minutes ago"));
  }

  #[wasm_bindgen_test]
  fn test_format_timestamp_hours_ago() {
    let ts = chrono::Utc::now().timestamp() - 7200; // 2 hours ago
    let result = format_timestamp(ts);
    assert!(result.contains("hours ago"));
  }

  #[wasm_bindgen_test]
  fn test_format_timestamp_days_ago() {
    let ts = chrono::Utc::now().timestamp() - 172800; // 2 days ago
    let result = format_timestamp(ts);
    assert!(result.contains("days ago"));
  }

  #[wasm_bindgen_test]
  fn test_format_timestamp_old_date() {
    // 2023-01-15 12:00:00 UTC
    let ts = 1673784000;
    let result = format_timestamp(ts);
    // Should display date format MM-DD HH:MM
    assert!(result.contains("-"));
    assert!(result.contains(":"));
  }
}
