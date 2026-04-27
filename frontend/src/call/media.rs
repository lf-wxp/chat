//! Thin, async wrappers around `navigator.mediaDevices.*`.
//!
//! These helpers isolate the `web_sys` / JS-interop surface from the
//! rest of the call subsystem so the higher-level `CallManager` can
//! stay readable and more easily unit-tested. All functions return
//! `Err(String)` on failure; the caller is expected to map the error
//! to a user-facing i18n key (`error.av001` etc.).

use js_sys::Reflect;
use message::types::MediaType;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
  DisplayMediaStreamConstraints, HtmlVideoElement, MediaStream, MediaStreamConstraints,
  MediaStreamTrack, MediaTrackConstraints,
};

use super::types::VideoProfile;

/// Acquire a local camera+microphone stream for the given call mode.
///
/// `Audio`-only calls request audio only; `Video` calls request both;
/// `ScreenShare` requests audio only here — the screen itself is
/// captured separately via [`acquire_display_stream`].
///
/// # Errors
/// Returns `Err` with an English description if `getUserMedia` fails
/// (e.g. the user denied permission, no device is available, or the
/// page is not served over HTTPS).
pub async fn acquire_user_media(media_type: MediaType) -> Result<MediaStream, String> {
  let window = web_sys::window().ok_or("No window available")?;
  let navigator = window.navigator();
  let devices = navigator
    .media_devices()
    .map_err(|e| format!("mediaDevices unavailable: {e:?}"))?;

  let constraints = MediaStreamConstraints::new();
  constraints.set_audio(&JsValue::TRUE);

  match media_type {
    MediaType::Audio | MediaType::ScreenShare => {
      constraints.set_video(&JsValue::FALSE);
    }
    MediaType::Video => {
      let video = MediaTrackConstraints::new();
      // Use the `HIGH` profile to stay aligned with the initial value
      // of `CallSignals::self_video_profile` (Round-4 consistency fix).
      // `HIGH` and `MEDIUM` currently share the same 720p@30 resolution
      // (see `VideoProfile`), so this change is semantically equivalent
      // but avoids a confusing "signal says HIGH, constraint says
      // MEDIUM" mismatch for future readers.
      apply_video_profile(&video, VideoProfile::HIGH);
      constraints.set_video(&JsValue::from(&video));
    }
  }

  let promise = devices
    .get_user_media_with_constraints(&constraints)
    .map_err(|e| format!("getUserMedia failed: {e:?}"))?;
  let stream = JsFuture::from(promise)
    .await
    .map_err(|e| format!("getUserMedia rejected: {e:?}"))?;
  stream
    .dyn_into::<MediaStream>()
    .map_err(|_| "getUserMedia did not return a MediaStream".to_string())
}

/// Acquire a *video-only* stream for the audio → video upgrade path.
///
/// Used by [`super::CallManager::toggle_camera`] when re-enabling the
/// camera mid-call: re-acquiring a combined audio+video stream would
/// hand us a second microphone track that the browser would expose as
/// a duplicate "tab is using microphone" indicator, while the original
/// audio sender on the PeerConnection is left untouched. Requesting
/// video only sidesteps the duplicate-track issue (M3 fix).
///
/// # Errors
/// Returns `Err` with an English description if `getUserMedia` fails
/// (e.g. the user denied permission, no camera is available, or the
/// page is not served over HTTPS).
pub async fn acquire_video_only_stream() -> Result<MediaStream, String> {
  let window = web_sys::window().ok_or("No window available")?;
  let navigator = window.navigator();
  let devices = navigator
    .media_devices()
    .map_err(|e| format!("mediaDevices unavailable: {e:?}"))?;

  let constraints = MediaStreamConstraints::new();
  constraints.set_audio(&JsValue::FALSE);
  let video = MediaTrackConstraints::new();
  apply_video_profile(&video, VideoProfile::HIGH);
  constraints.set_video(&JsValue::from(&video));

  let promise = devices
    .get_user_media_with_constraints(&constraints)
    .map_err(|e| format!("getUserMedia failed: {e:?}"))?;
  let stream = JsFuture::from(promise)
    .await
    .map_err(|e| format!("getUserMedia rejected: {e:?}"))?;
  stream
    .dyn_into::<MediaStream>()
    .map_err(|_| "getUserMedia did not return a MediaStream".to_string())
}

/// Acquire a screen-capture stream via `getDisplayMedia`.
///
/// The returned stream contains a single video track representing the
/// shared surface and, on browsers that support it, an audio track if
/// the user opted to share system audio.
///
/// # Errors
/// Returns `Err` with an English description if the user cancels the
/// picker dialog or the browser rejects the request.
pub async fn acquire_display_stream() -> Result<MediaStream, String> {
  let window = web_sys::window().ok_or("No window available")?;
  let navigator = window.navigator();
  let devices = navigator
    .media_devices()
    .map_err(|e| format!("mediaDevices unavailable: {e:?}"))?;

  let constraints = DisplayMediaStreamConstraints::new();
  constraints.set_video(&JsValue::TRUE);
  constraints.set_audio(&JsValue::TRUE);

  let promise = devices
    .get_display_media_with_constraints(&constraints)
    .map_err(|e| format!("getDisplayMedia failed: {e:?}"))?;
  let stream = JsFuture::from(promise)
    .await
    .map_err(|e| format!("getDisplayMedia rejected: {e:?}"))?;
  stream
    .dyn_into::<MediaStream>()
    .map_err(|_| "getDisplayMedia did not return a MediaStream".to_string())
}

/// Apply a [`VideoProfile`] to a set of media-track constraints.
pub fn apply_video_profile(constraints: &MediaTrackConstraints, profile: VideoProfile) {
  // `width` / `height` / `frameRate` accept plain numbers; the spec
  // then resolves them to `{ ideal: N }` internally, so we do not need
  // to build explicit `ConstrainLongRange` dictionaries.
  constraints.set_width(&JsValue::from_f64(f64::from(profile.width)));
  constraints.set_height(&JsValue::from_f64(f64::from(profile.height)));
  constraints.set_frame_rate(&JsValue::from_f64(f64::from(profile.frame_rate)));
}

/// Re-constrain an existing video `MediaStreamTrack` to a new profile.
///
/// Called by the quality-downgrade controller when network conditions
/// change. Uses `applyConstraints` rather than re-acquiring the stream
/// so existing `RtcRtpSender`s continue to flow without re-negotiation.
///
/// # Errors
/// Returns `Err` if the track rejects the constraints.
pub async fn retarget_video_track(
  track: &MediaStreamTrack,
  profile: VideoProfile,
) -> Result<(), String> {
  let constraints = MediaTrackConstraints::new();
  apply_video_profile(&constraints, profile);
  let promise = track
    .apply_constraints_with_constraints(&constraints)
    .map_err(|e| format!("applyConstraints threw: {e:?}"))?;
  JsFuture::from(promise)
    .await
    .map_err(|e| format!("applyConstraints rejected: {e:?}"))?;
  Ok(())
}

/// Stop every track in a `MediaStream`.
///
/// Call this when tearing down a call so the browser's "tab is using
/// microphone" indicator disappears promptly.
pub fn stop_stream(stream: &MediaStream) {
  let tracks = stream.get_tracks();
  for i in 0..tracks.length() {
    if let Some(track) = tracks.get(i).dyn_ref::<MediaStreamTrack>() {
      track.stop();
    }
  }
}

/// Return the first audio track of a stream, if any.
#[must_use]
pub fn first_audio_track(stream: &MediaStream) -> Option<MediaStreamTrack> {
  stream.get_audio_tracks().get(0).dyn_into().ok()
}

/// Return the first video track of a stream, if any.
#[must_use]
pub fn first_video_track(stream: &MediaStream) -> Option<MediaStreamTrack> {
  stream.get_video_tracks().get(0).dyn_into().ok()
}

/// Request Picture-in-Picture mode for an `HTMLVideoElement`.
///
/// Implemented via `Reflect::get` so we do not have to depend on a
/// particular `web_sys` feature set for the PiP API surface.
///
/// # Errors
/// Returns `Err` if the element does not support PiP or the request is
/// rejected (browser policy, user gesture missing, etc.).
pub async fn request_picture_in_picture(video: &HtmlVideoElement) -> Result<(), String> {
  let request_fn = Reflect::get(video, &JsValue::from_str("requestPictureInPicture"))
    .map_err(|_| "requestPictureInPicture not available".to_string())?;
  let function = request_fn
    .dyn_into::<js_sys::Function>()
    .map_err(|_| "requestPictureInPicture is not a function".to_string())?;
  let promise_val = function
    .call0(video)
    .map_err(|e| format!("requestPictureInPicture threw: {e:?}"))?;
  let promise = promise_val
    .dyn_into::<js_sys::Promise>()
    .map_err(|_| "requestPictureInPicture did not return a Promise".to_string())?;
  JsFuture::from(promise)
    .await
    .map_err(|e| format!("requestPictureInPicture rejected: {e:?}"))?;
  Ok(())
}

/// Exit Picture-in-Picture if the document is currently showing it.
///
/// Silently succeeds if PiP is not active.
pub async fn exit_picture_in_picture() -> Result<(), String> {
  let window = web_sys::window().ok_or("No window available")?;
  let document = window.document().ok_or("No document available")?;

  // `document.pictureInPictureElement` is only exposed on documents
  // that support PiP; use Reflect so the code still compiles against
  // web-sys builds that pre-date the API.
  let pip_el =
    Reflect::get(&document, &JsValue::from_str("pictureInPictureElement")).unwrap_or(JsValue::NULL);
  if pip_el.is_null() || pip_el.is_undefined() {
    return Ok(());
  }

  let exit_fn = Reflect::get(&document, &JsValue::from_str("exitPictureInPicture"))
    .map_err(|_| "exitPictureInPicture not available".to_string())?;
  let function = exit_fn
    .dyn_into::<js_sys::Function>()
    .map_err(|_| "exitPictureInPicture is not a function".to_string())?;
  let promise_val = function
    .call0(&document)
    .map_err(|e| format!("exitPictureInPicture threw: {e:?}"))?;
  let promise = promise_val
    .dyn_into::<js_sys::Promise>()
    .map_err(|_| "exitPictureInPicture did not return a Promise".to_string())?;
  JsFuture::from(promise)
    .await
    .map_err(|e| format!("exitPictureInPicture rejected: {e:?}"))?;
  Ok(())
}

/// Attach a `MediaStream` to a `<video>` element, enabling autoplay.
///
/// Uses `Reflect::set` for the `srcObject` property because `web_sys`
/// does not expose the DOM setter directly.
///
/// # Errors
/// Returns `Err` if the property assignment throws.
pub fn attach_stream_to_video(
  video: &HtmlVideoElement,
  stream: Option<&MediaStream>,
) -> Result<(), String> {
  let value = stream.map_or(JsValue::NULL, JsValue::from);
  Reflect::set(video, &JsValue::from_str("srcObject"), &value)
    .map_err(|e| format!("Failed to set srcObject: {e:?}"))?;
  video.set_autoplay(true);
  Ok(())
}
