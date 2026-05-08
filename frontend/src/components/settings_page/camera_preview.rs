//! Camera preview component for the AV settings section (A14).
//!
//! Shows a live video stream from the selected camera when the user
//! clicks "Preview Camera". Stops the stream automatically on
//! component unmount or when the user clicks "Stop Preview".

use crate::i18n;
use crate::settings::use_settings_state;
use icondata as i;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_i18n::{t, t_string};
use leptos_icons::Icon;
use wasm_bindgen::JsCast;

/// Camera preview component (Req 13.1.4 — device test mechanism).
#[component]
pub(super) fn CameraPreview() -> impl IntoView {
  let i18n = i18n::use_i18n();
  let settings = use_settings_state();
  let previewing: RwSignal<bool> = RwSignal::new(false);
  let denied: RwSignal<bool> = RwSignal::new(false);
  let video_ref: NodeRef<leptos::html::Video> = NodeRef::new();
  let stream_handle: StoredValue<Option<web_sys::MediaStream>> = StoredValue::new(None);

  let start_preview = move |_| {
    previewing.set(true);
    denied.set(false);
    let device_id = settings.get().default_camera.clone();
    let video_el = video_ref.get();
    spawn_local(async move {
      let window = leptos_use::use_window();
      let Some(window) = window.as_ref() else {
        denied.set(true);
        previewing.set(false);
        return;
      };
      let navigator = window.navigator();
      let media_devices = match navigator.media_devices() {
        Ok(m) => m,
        Err(_) => {
          denied.set(true);
          previewing.set(false);
          return;
        }
      };

      // Build video constraints, optionally targeting the selected device.
      let constraints = web_sys::MediaStreamConstraints::new();
      if let Some(id) = device_id {
        let video_constraints = js_sys::Object::new();
        let _ = js_sys::Reflect::set(
          &video_constraints,
          &wasm_bindgen::JsValue::from_str("deviceId"),
          &wasm_bindgen::JsValue::from_str(&id),
        );
        constraints.set_video(&video_constraints.into());
      } else {
        constraints.set_video(&wasm_bindgen::JsValue::TRUE);
      }
      constraints.set_audio(&wasm_bindgen::JsValue::FALSE);

      let Ok(promise) = media_devices.get_user_media_with_constraints(&constraints) else {
        denied.set(true);
        previewing.set(false);
        return;
      };
      let Ok(stream_val) = wasm_bindgen_futures::JsFuture::from(promise).await else {
        denied.set(true);
        previewing.set(false);
        return;
      };
      let Ok(stream) = stream_val.dyn_into::<web_sys::MediaStream>() else {
        denied.set(true);
        previewing.set(false);
        return;
      };

      // Attach the stream to the <video> element.
      if let Some(el) = video_el {
        let el: &web_sys::HtmlVideoElement = &el;
        el.set_src_object(Some(&stream));
        let _ = el.play();
      }
      stream_handle.set_value(Some(stream));
    });
  };

  let stop_preview = move |_| {
    stream_handle.with_value(|opt| {
      if let Some(stream) = opt.as_ref() {
        let tracks = stream.get_tracks();
        for i in 0..tracks.length() {
          if let Some(track) = tracks.get(i).dyn_ref::<web_sys::MediaStreamTrack>() {
            track.stop();
          }
        }
      }
    });
    stream_handle.set_value(None);
    if let Some(el) = video_ref.get() {
      let el: &web_sys::HtmlVideoElement = &el;
      el.set_src_object(None::<&web_sys::MediaStream>);
    }
    previewing.set(false);
  };

  // Cleanup on unmount.
  on_cleanup(move || {
    stream_handle.with_value(|opt| {
      if let Some(stream) = opt.as_ref() {
        let tracks = stream.get_tracks();
        for i in 0..tracks.length() {
          if let Some(track) = tracks.get(i).dyn_ref::<web_sys::MediaStreamTrack>() {
            track.stop();
          }
        }
      }
    });
  });

  view! {
    <div class="settings-row settings-camera-preview-row">
      <Show when=move || previewing.get()>
        <div class="settings-camera-preview">
          <video
            node_ref=video_ref
            class="settings-camera-video"
            autoplay
            muted
            playsinline
            aria-label=move || t_string!(i18n, settings.camera_preview)
          ></video>
          <button
            class="btn-ghost settings-action"
            on:click=stop_preview
            data-testid="stop-camera-preview"
          >
            <Icon icon=i::LuVideoOff />
            <span>{t!(i18n, settings.stop_camera_preview)}</span>
          </button>
        </div>
      </Show>
      <Show when=move || !previewing.get() && !denied.get()>
        <button
          class="btn-primary settings-action"
          on:click=start_preview
          data-testid="start-camera-preview"
        >
          <Icon icon=i::LuCamera />
          <span>{t!(i18n, settings.start_camera_preview)}</span>
        </button>
      </Show>
      <Show when=move || denied.get()>
        <p class="settings-hint">{t!(i18n, settings.camera_preview_denied)}</p>
      </Show>
    </div>
  }
}
