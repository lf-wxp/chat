//! Device selector dropdown component.
//!
//! Renders a `<select>` for a specific device kind (camera, microphone,
//! or speaker), backed by the shared `DeviceCache`. Extracted from
//! `av_section.rs` to satisfy the "one component per file" rule.

use super::av_helpers::{DeviceEntry, DeviceKind};
use crate::i18n;
use crate::settings::use_settings_state;
use leptos::prelude::*;
use leptos_i18n::t;

/// Device selector dropdown for camera / microphone / speaker.
#[component]
pub(super) fn DeviceSelect(
  kind: DeviceKind,
  filtered_devices: Memo<Vec<DeviceEntry>>,
  label: AnyView,
) -> impl IntoView {
  let i18n = i18n::use_i18n();
  let settings = use_settings_state();

  let selected_value = Memo::new(move |_| {
    let snapshot = settings.get();
    match kind {
      DeviceKind::Camera => snapshot.default_camera.unwrap_or_default(),
      DeviceKind::Microphone => snapshot.default_microphone.unwrap_or_default(),
      DeviceKind::Speaker => snapshot.default_speaker.unwrap_or_default(),
    }
  });

  let on_change = move |ev| {
    let value = event_target_value(&ev);
    let value = if value.is_empty() { None } else { Some(value) };
    settings.update(|s| match kind {
      DeviceKind::Camera => s.default_camera = value,
      DeviceKind::Microphone => s.default_microphone = value,
      DeviceKind::Speaker => s.default_speaker = value,
    });
  };

  view! {
    <div class="settings-row">
      <label class="settings-label">{label}</label>
      <select
        class="settings-select"
        prop:value=move || selected_value.get()
        on:change=on_change
        data-testid=match kind {
          DeviceKind::Camera => "select-camera",
          DeviceKind::Microphone => "select-microphone",
          DeviceKind::Speaker => "select-speaker",
        }
      >
        <option value="">{t!(i18n, settings.default_device_system)}</option>
        <For
          each=move || filtered_devices.get()
          key=|entry: &DeviceEntry| entry.device_id.clone()
          children=move |entry: DeviceEntry| {
            let id = entry.device_id.clone();
            let label = entry.label.clone();
            view! { <option value=id>{label}</option> }
          }
        />
      </select>
    </div>
  }
}
