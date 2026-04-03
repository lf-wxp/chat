//! Picture-in-Picture floating window manager
//!
//! Wraps the browser's native PiP API to support popping call videos into floating windows.
//! Since web-sys does not directly bind the PiP API, we manually call it via `js_sys::Reflect`.

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

/// PiP status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PipStatus {
  /// Inactive
  #[default]
  Inactive,
  /// Entering PiP
  Entering,
  /// PiP active
  Active,
}

/// PiP manager
///
/// Manages entering and exiting Picture-in-Picture floating windows.
#[derive(Clone)]
pub struct PipManager {
  /// Current PiP status
  status: RwSignal<PipStatus>,
  /// ID of the video element currently in PiP mode
  active_video_id: StoredValue<Option<String>>,
}

impl PipManager {
  /// Create and provide to context
  pub fn provide() {
    let manager = Self {
      status: RwSignal::new(PipStatus::Inactive),
      active_video_id: StoredValue::new(None),
    };
    provide_context(manager);
  }

  /// Get from context
  pub fn use_manager() -> Self {
    use_context::<Self>().expect("PipManager not provided")
  }

  /// Get current PiP status signal (read-only)
  pub fn status(&self) -> ReadSignal<PipStatus> {
    self.status.read_only()
  }

  /// Check if browser supports PiP
  pub fn is_supported() -> bool {
    web_sys::window()
      .and_then(|w| w.document())
      .is_some_and(|doc| {
        js_sys::Reflect::get(&doc, &"pictureInPictureEnabled".into())
          .ok()
          .and_then(|v| v.as_bool())
          .unwrap_or(false)
      })
  }

  /// Enter PiP mode
  ///
  /// `video_id` is the DOM ID of the target `<video>` element.
  /// If another video is already in PiP, it will exit first before entering the new one.
  pub fn enter(&self, video_id: &str) {
    if !Self::is_supported() {
      web_sys::console::warn_1(&"Browser does not support Picture-in-Picture".into());
      return;
    }

    let video_id = video_id.to_string();
    let status = self.status;
    let active_video_id = self.active_video_id;

    // Mark as entering
    status.set(PipStatus::Entering);

    wasm_bindgen_futures::spawn_local(async move {
      // If PiP is already active, exit it first
      if let Err(e) = exit_pip_internal().await {
        web_sys::console::warn_1(&format!("Failed to exit existing PiP (ignored): {e:?}").into());
      }

      // Get target video element
      let Some(video) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(&video_id))
      else {
        web_sys::console::error_1(&format!("PiP: Video element not found #{video_id}").into());
        status.set(PipStatus::Inactive);
        return;
      };

      let video: web_sys::HtmlVideoElement = video.unchecked_into();

      // Call requestPictureInPicture()
      match request_pip(&video).await {
        Ok(_pip_window) => {
          active_video_id.set_value(Some(video_id.clone()));
          status.set(PipStatus::Active);

          // Listen for leavepictureinpicture event
          let status_leave = status;
          let active_id_leave = active_video_id;
          let on_leave = Closure::<dyn Fn()>::new(move || {
            status_leave.set(PipStatus::Inactive);
            active_id_leave.set_value(None);
          });
          video
            .add_event_listener_with_callback(
              "leavepictureinpicture",
              on_leave.as_ref().unchecked_ref(),
            )
            .ok();
          on_leave.forget();

          web_sys::console::log_1(&format!("PiP: Entered floating window (#{video_id})").into());
        }
        Err(e) => {
          web_sys::console::error_1(&format!("PiP: Failed to enter: {e:?}").into());
          status.set(PipStatus::Inactive);
        }
      }
    });
  }

  /// Exit PiP mode
  pub fn exit(&self) {
    let status = self.status;
    let active_video_id = self.active_video_id;

    wasm_bindgen_futures::spawn_local(async move {
      match exit_pip_internal().await {
        Ok(()) => {
          status.set(PipStatus::Inactive);
          active_video_id.set_value(None);
          web_sys::console::log_1(&"PiP: Exited floating window".into());
        }
        Err(e) => {
          web_sys::console::warn_1(&format!("PiP: Failed to exit: {e:?}").into());
        }
      }
    });
  }

  /// Toggle PiP mode
  pub fn toggle(&self, video_id: &str) {
    if self.status.get_untracked() == PipStatus::Active {
      self.exit();
    } else {
      self.enter(video_id);
    }
  }

  /// Check if currently in PiP mode
  pub fn is_active(&self) -> bool {
    self.status.get_untracked() == PipStatus::Active
  }

  /// Get current PiP video element ID
  #[allow(dead_code)]
  pub fn active_video_id(&self) -> Option<String> {
    self.active_video_id.get_value()
  }
}

/// Call video.requestPictureInPicture()
async fn request_pip(video: &web_sys::HtmlVideoElement) -> Result<JsValue, JsValue> {
  let func = js_sys::Reflect::get(video, &"requestPictureInPicture".into())?;
  let func: js_sys::Function = func.dyn_into()?;
  let promise: js_sys::Promise = func.call0(video)?.dyn_into()?;
  JsFuture::from(promise).await
}

/// Call document.exitPictureInPicture()
async fn exit_pip_internal() -> Result<(), JsValue> {
  let document = web_sys::window()
    .and_then(|w| w.document())
    .ok_or_else(|| JsValue::from_str("Unable to get document"))?;

  // Check if there's an active PiP element
  let pip_element = js_sys::Reflect::get(&document, &"pictureInPictureElement".into())?;
  if pip_element.is_null() || pip_element.is_undefined() {
    return Ok(()); // No active PiP, nothing to exit
  }

  let func = js_sys::Reflect::get(&document, &"exitPictureInPicture".into())?;
  let func: js_sys::Function = func.dyn_into()?;
  let promise: js_sys::Promise = func.call0(&document)?.dyn_into()?;
  JsFuture::from(promise).await?;
  Ok(())
}
