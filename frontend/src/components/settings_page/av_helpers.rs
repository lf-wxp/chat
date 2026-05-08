//! Audio/video device enumeration helpers.
//!
//! Extracted from `av_section.rs` for testability and separation of
//! concerns (O-6). Contains the device-enumeration logic, device
//! types, and the `DeviceCache` context type.

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{MediaDeviceInfo, MediaStreamConstraints};

/// One enumerated input/output device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeviceEntry {
  pub device_id: String,
  pub label: String,
  pub kind: DeviceKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DeviceKind {
  Camera,
  Microphone,
  Speaker,
}

/// Cached device list that outlives the `<Show when=open>` toggle so
/// `AvSection` does not re-enumerate devices on every drawer
/// open/close cycle (V3-M-3). Created in `SettingsPage` and provided
/// via `provide_context`. Loading + permission state are also kept
/// here so the async `enumerate_devices` task can safely write to
/// them after `AvSection` has been unmounted (e.g. user closed the
/// drawer before the `getUserMedia` promise resolved). Storing the
/// flags on component-scoped signals caused `panic_already_borrowed`
/// / `Get::get` panics when the owner was disposed mid-flight.
#[derive(Clone, Copy)]
pub(super) struct DeviceCache {
  pub devices: RwSignal<Vec<DeviceEntry>>,
  /// Whether the initial enumeration has been performed at least once.
  pub initialised: RwSignal<bool>,
  pub load_error: RwSignal<Option<String>>,
  pub needs_permission: RwSignal<bool>,
  pub permission_denied: RwSignal<bool>,
}

impl DeviceCache {
  pub fn new() -> Self {
    Self {
      devices: RwSignal::new(Vec::new()),
      initialised: RwSignal::new(false),
      load_error: RwSignal::new(None),
      needs_permission: RwSignal::new(false),
      permission_denied: RwSignal::new(false),
    }
  }
}

/// Generate a fallback label for an unlabelled device.
pub(super) fn default_label(kind: DeviceKind, idx: usize) -> String {
  let prefix = match kind {
    DeviceKind::Camera => "Camera",
    DeviceKind::Microphone => "Microphone",
    DeviceKind::Speaker => "Speaker",
  };
  format!("{prefix} {}", idx + 1)
}

/// Enumerate devices via `navigator.mediaDevices.enumerateDevices()`.
///
/// When `prompt_permission` is `true`, first requests the user-media
/// permission so the subsequent enumeration returns populated device
/// labels. When `false`, the browser returns devices with empty
/// labels whenever permission has not yet been granted — the UI
/// surfaces a "Request Device Permission" button instead of silently
/// triggering a prompt.
pub(super) fn enumerate_devices(
  window: leptos_use::UseWindow,
  devices: RwSignal<Vec<DeviceEntry>>,
  load_error: RwSignal<Option<String>>,
  needs_permission: RwSignal<bool>,
  permission_denied: RwSignal<bool>,
  prompt_permission: bool,
) {
  let Some(window) = window.as_ref().cloned() else {
    load_error.set(Some("no window".to_string()));
    return;
  };
  spawn_local(async move {
    let navigator = window.navigator();
    let media_devices = match navigator.media_devices() {
      Ok(m) => m,
      Err(_) => {
        load_error.set(Some("unavailable".to_string()));
        return;
      }
    };

    if prompt_permission {
      let constraints = MediaStreamConstraints::new();
      constraints.set_audio(&wasm_bindgen::JsValue::TRUE);
      constraints.set_video(&wasm_bindgen::JsValue::TRUE);
      if let Ok(promise) = media_devices.get_user_media_with_constraints(&constraints) {
        match JsFuture::from(promise).await {
          Ok(stream_val) => {
            needs_permission.set(false);
            permission_denied.set(false);
            if let Ok(stream) = stream_val.dyn_into::<web_sys::MediaStream>() {
              // Stop tracks immediately — we only wanted the labels.
              let tracks = stream.get_tracks();
              for i in 0..tracks.length() {
                if let Some(track) = tracks.get(i).dyn_ref::<web_sys::MediaStreamTrack>() {
                  track.stop();
                }
              }
            }
          }
          Err(_) => {
            needs_permission.set(true);
            permission_denied.set(true);
          }
        }
      }
    }

    let promise = match media_devices.enumerate_devices() {
      Ok(p) => p,
      Err(_) => {
        load_error.set(Some("enumerate_failed".to_string()));
        return;
      }
    };

    let list = match JsFuture::from(promise).await {
      Ok(list) => list,
      Err(_) => {
        load_error.set(Some("enumerate_rejected".to_string()));
        return;
      }
    };

    let array = js_sys::Array::from(&list);
    let mut parsed: Vec<DeviceEntry> = Vec::with_capacity(array.length() as usize);
    let mut any_unlabelled = false;
    for (idx, value) in array.iter().enumerate() {
      let Ok(device) = value.dyn_into::<MediaDeviceInfo>() else {
        continue;
      };
      let kind_str = js_sys::Reflect::get(&device, &wasm_bindgen::JsValue::from_str("kind"))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_default();
      let kind = match kind_str.as_str() {
        "videoinput" => DeviceKind::Camera,
        "audioinput" => DeviceKind::Microphone,
        "audiooutput" => DeviceKind::Speaker,
        _ => continue,
      };
      let raw_label = device.label();
      if raw_label.is_empty() {
        any_unlabelled = true;
      }
      let label = if raw_label.is_empty() {
        default_label(kind, idx)
      } else {
        raw_label
      };
      parsed.push(DeviceEntry {
        device_id: device.device_id(),
        label,
        kind,
      });
    }

    // If at least one device is reported but any of them lacks a
    // label, the user has not granted permission yet — surface the
    // "Request Device Permission" button rather than silently
    // showing "Camera 1".
    needs_permission.set(any_unlabelled && !parsed.is_empty());
    load_error.set(None);
    devices.set(parsed);
  });
}

// ---------------------------------------------------------------------------
// Unit tests — types that can be exercised without a browser runtime.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn device_kind_default_label_generates_expected_names() {
    assert_eq!(default_label(DeviceKind::Camera, 0), "Camera 1");
    assert_eq!(default_label(DeviceKind::Camera, 1), "Camera 2");
    assert_eq!(default_label(DeviceKind::Microphone, 0), "Microphone 1");
    assert_eq!(default_label(DeviceKind::Speaker, 2), "Speaker 3");
  }

  #[test]
  fn device_entry_equality_and_clone() {
    let a = DeviceEntry {
      device_id: "id-1".into(),
      label: "Label A".into(),
      kind: DeviceKind::Camera,
    };
    let b = DeviceEntry {
      device_id: "id-1".into(),
      label: "Label A".into(),
      kind: DeviceKind::Camera,
    };
    let c = DeviceEntry {
      device_id: "id-2".into(),
      label: "Label B".into(),
      kind: DeviceKind::Microphone,
    };
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_eq!(a.clone(), a);
  }

  #[test]
  fn device_kind_debug_and_clone() {
    let kind = DeviceKind::Microphone;
    let dbg = format!("{:?}", kind);
    assert!(!dbg.is_empty());
    assert_eq!(kind, kind.clone());
  }
}
