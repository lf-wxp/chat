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
  DisplayMediaStreamConstraints, HtmlMediaElement, HtmlVideoElement, MediaStream,
  MediaStreamConstraints, MediaStreamTrack, MediaTrackConstraints,
};

use super::types::VideoProfile;
use crate::settings::{UserSettings, VideoQualityPref, load_snapshot};

/// Translate the user's persisted [`VideoQualityPref`] into a
/// concrete [`VideoProfile`]. The runtime quality controller may
/// further degrade this profile when network conditions worsen — the
/// preference is treated as an upper bound.
#[must_use]
pub fn baseline_video_profile(pref: VideoQualityPref) -> VideoProfile {
  match pref {
    VideoQualityPref::Auto => VideoProfile::HIGH,
    VideoQualityPref::Low => VideoProfile::LOW,
    VideoQualityPref::Standard => VideoProfile::HIGH,
    VideoQualityPref::High => VideoProfile {
      width: 1920,
      height: 1080,
      frame_rate: 30,
    },
  }
}

/// Apply the user's preferred audio-input device id to the audio
/// constraints, when one is configured. No-op for `None`.
fn apply_audio_device(audio: &MediaTrackConstraints, settings: &UserSettings) {
  if let Some(device_id) = settings.default_microphone.as_deref()
    && !device_id.is_empty()
  {
    audio.set_device_id(&JsValue::from_str(device_id));
  }
}

/// Apply the user's preferred camera device id to the video
/// constraints, when one is configured.
fn apply_video_device(video: &MediaTrackConstraints, settings: &UserSettings) {
  if let Some(device_id) = settings.default_camera.as_deref()
    && !device_id.is_empty()
  {
    video.set_device_id(&JsValue::from_str(device_id));
  }
}

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

  let settings = load_snapshot();
  let constraints = MediaStreamConstraints::new();
  // Apply the user's preferred microphone (Req 13.1.5).
  let audio = MediaTrackConstraints::new();
  apply_audio_device(&audio, &settings);
  constraints.set_audio(&JsValue::from(&audio));

  match media_type {
    MediaType::Audio | MediaType::ScreenShare => {
      constraints.set_video(&JsValue::FALSE);
    }
    MediaType::Video => {
      let video = MediaTrackConstraints::new();
      // Apply the user's preferred camera (Req 13.1.4) and the
      // baseline profile derived from the video-quality preference
      // (Req 13.1.6). The runtime quality controller may downgrade
      // this profile later via `applyConstraints`.
      apply_video_device(&video, &settings);
      apply_video_profile(&video, baseline_video_profile(settings.video_quality));
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
/// video only sidesteps the duplicate-track issue.
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

  let settings = load_snapshot();
  let constraints = MediaStreamConstraints::new();
  constraints.set_audio(&JsValue::FALSE);
  let video = MediaTrackConstraints::new();
  apply_video_device(&video, &settings);
  apply_video_profile(&video, baseline_video_profile(settings.video_quality));
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

  // IMPORTANT: When `audio: true` is set in getDisplayMedia constraints,
  // Chrome restricts the picker to only show browser tabs (because system
  // audio capture is only supported for tab capture). To allow the user
  // to share windows and entire screens, we set `audio` to a preference
  // object that makes it optional, or omit it entirely and let the
  // browser offer the "Share audio" checkbox when applicable (tab mode).
  //
  // Setting video to `true` allows all surface types (monitor, window,
  // browser tab). The user can still opt into audio sharing when they
  // pick a browser tab — Chrome shows the "Share tab audio" checkbox.
  let constraints = DisplayMediaStreamConstraints::new();
  constraints.set_video(&JsValue::TRUE);
  // Do NOT set audio to true — this restricts Chrome to tab-only mode.
  // Instead, use `{ audio: { optional: [] } }` or simply omit audio
  // to let the browser decide based on the selected surface type.
  // On Chromium, omitting audio still shows the "Share audio" checkbox
  // for tab captures.

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
/// does not expose the DOM setter directly. Also applies the user's
/// preferred output device and speaker volume (Req 13.1.2 / 13.1.5)
/// so playback honours the persisted settings.
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
  let media_el: &HtmlMediaElement = video.as_ref();
  apply_speaker_settings(media_el);
  Ok(())
}

/// Apply the persisted speaker preferences (output device + volume)
/// to a media element. Silently no-ops on browsers that do not expose
/// `setSinkId` (Firefox, Safari < 17). Volume is always set since
/// every browser supports `HTMLMediaElement.volume`.
pub fn apply_speaker_settings(media: &HtmlMediaElement) {
  let settings = load_snapshot();
  // Volume is a 0.0 – 1.0 scalar — sanitised on load.
  media.set_volume(f64::from(settings.speaker_volume));

  if let Some(sink_id) = settings.default_speaker.as_deref()
    && !sink_id.is_empty()
    && let Ok(set_sink_fn) = Reflect::get(media, &JsValue::from_str("setSinkId"))
    && let Ok(function) = set_sink_fn.dyn_into::<js_sys::Function>()
    && let Ok(promise_val) = function.call1(media, &JsValue::from_str(sink_id))
    && let Ok(promise) = promise_val.dyn_into::<js_sys::Promise>()
  {
    // Fire-and-forget: rejection (e.g. unsupported sink) is swallowed
    // because the user can correct the device choice from settings.
    wasm_bindgen_futures::spawn_local(async move {
      let _ = JsFuture::from(promise).await;
    });
  }
}

/// Capture a `MediaStream` from a `<video>` element using the
/// `captureStream()` API (Req 12.3 §8).
///
/// Tries `captureStream()` first (Chrome, Edge), then falls back to
/// `mozCaptureStream()` (Firefox). Returns an error message suitable
/// for display when neither API is available.
///
/// # Errors
/// Returns `Err` when the browser does not support video stream
/// capture (e.g. Safari < 17, or non-HTTPS contexts).
pub fn capture_stream_from_video(video: &HtmlVideoElement) -> Result<MediaStream, String> {
  // Try standard `captureStream()` first.
  if let Ok(func) = Reflect::get(video, &JsValue::from_str("captureStream"))
    && let Ok(function) = func.dyn_into::<js_sys::Function>()
    && let Ok(result) = function.call0(video)
    && let Ok(stream) = result.dyn_into::<MediaStream>()
  {
    return Ok(stream);
  }

  // Fallback: Firefox uses `mozCaptureStream()`.
  if let Ok(func) = Reflect::get(video, &JsValue::from_str("mozCaptureStream"))
    && let Ok(function) = func.dyn_into::<js_sys::Function>()
    && let Ok(result) = function.call0(video)
    && let Ok(stream) = result.dyn_into::<MediaStream>()
  {
    return Ok(stream);
  }

  Err("Your browser does not support video stream capture. Please use Chrome or Edge.".to_string())
}
