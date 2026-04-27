//! Media toggle operations: mute, camera, screen-share, PiP.
//!
//! These methods mutate the local capture state and broadcast
//! `MediaStateUpdate` to remote peers so their tiles can render
//! the correct icons (Req 3.5 / 7.1).

use leptos::task::spawn_local;

use super::*;
use crate::utils::format_duration;

impl CallManager {
  /// Toggle the local microphone. Returns the new enabled state.
  pub fn toggle_mute(&self) -> bool {
    let mut new_enabled = false;
    self.signals.local_media.update(|media| {
      media.mic_enabled = !media.mic_enabled;
      new_enabled = media.mic_enabled;
    });
    if let Some(stream) = self.signals.local_stream.get_untracked()
      && let Some(track) = media::first_audio_track(&stream)
    {
      track.set_enabled(new_enabled);
    }
    // Notify remote peers so their tiles can render a muted icon
    // (Req 3.5 — P1-New fix).
    self.broadcast_media_state();
    new_enabled
  }

  /// Toggle the local camera. Returns the new enabled state.
  ///
  /// When the call started in `Audio`-only mode there is no live video
  /// track to enable; in that case this method asynchronously requests
  /// `getUserMedia({video:true})`, attaches the new track to every
  /// connected peer via `replace_local_track`, and updates the local
  /// preview. The browser's `onnegotiationneeded` callback (installed
  /// in [`crate::webrtc::WebRtcManager`]) then drives a fresh SDP
  /// round-trip so the remote side actually receives the video (Req
  /// 3.4 / 7.2 — voice → video without re-establishing the connection).
  ///
  /// When turning the camera **off** (P1-New-1 fix) the video track is
  /// stopped and `sender.replaceTrack(null)` is called on every peer
  /// so the remote `<video>` element clears and the tile falls back to
  /// the avatar placeholder instead of freezing on the last frame (Req
  /// 3.6 — remote SHALL display an avatar placeholder).
  ///
  /// # Errors
  /// Returns `Err` if the camera permission is denied or the user
  /// cancels the picker.
  pub async fn toggle_camera(&self) -> Result<bool, String> {
    let new_enabled = !self.signals.local_media.get_untracked().camera_enabled;
    self
      .signals
      .local_media
      .update(|media| media.camera_enabled = new_enabled);

    let stream = self.signals.local_stream.get_untracked();

    // Turn-off path: stop the capture track and clear the remote sender
    // so the remote tile falls back to the avatar placeholder (Req 3.6 /
    // P1-New-1 fix). We also broadcast the new media state so remote
    // UIs can show a "camera off" icon (Req 3.5 / 7.1).
    if !new_enabled {
      if let Some(ref stream) = stream
        && let Some(track) = media::first_video_track(stream)
      {
        track.stop();
      }
      let webrtc = self.webrtc.borrow().clone();
      if let Some(webrtc) = webrtc {
        webrtc.clear_local_track_of_kind("video").await;
      }
      self.broadcast_media_state();
      return Ok(false);
    }

    // Turn-on path: if a video track already exists on the local stream
    // (e.g. the call started in Video mode and we are just re-enabling
    // it), flip its `enabled` flag and we're done.
    if let Some(ref stream) = stream
      && let Some(track) = media::first_video_track(stream)
    {
      track.set_enabled(true);
      self.broadcast_media_state();
      return Ok(true);
    }

    // Voice-only mode upgrading to video: acquire the camera and add
    // its video track to every connected peer without rebuilding the
    // PeerConnection (Req 7.2 — no re-negotiation handshake; the
    // browser's `onnegotiationneeded` will fire automatically).
    //
    // M3 fix: request a *video-only* stream rather than a combined
    // audio+video one. The audio sender on the PeerConnection is
    // already wired up from the original `acquire_user_media` call;
    // adopting a second microphone track here would surface a
    // duplicate "tab is using microphone" indicator without ever
    // being published.
    let new_video_stream = media::acquire_video_only_stream().await.map_err(|e| {
      // Roll back the optimistic UI update.
      self
        .signals
        .local_media
        .update(|m| m.camera_enabled = false);
      format!("Failed to acquire camera: {e}")
    })?;

    let video_track = match media::first_video_track(&new_video_stream) {
      Some(t) => t,
      None => {
        self
          .signals
          .local_media
          .update(|m| m.camera_enabled = false);
        return Err("No video track in acquired stream".to_string());
      }
    };

    // Clone the manager out of the RefCell before awaiting so we do
    // not hold the borrow across the `replace_local_track` suspension
    // point (clippy::await_holding_refcell_ref).
    let webrtc = self.webrtc.borrow().clone();
    if let Some(webrtc) = webrtc
      && let Err(e) = webrtc
        .replace_local_track(&video_track, &new_video_stream)
        .await
    {
      web_sys::console::warn_1(
        &format!("[call] replace_local_track for new video failed: {e}").into(),
      );
    }

    // Merge the new video track into the existing audio-only stream
    // so the local preview surfaces both. We deliberately keep the
    // original `MediaStream` object — and therefore the original
    // audio track — instead of swapping wholesale, which would
    // strand the audio sender on the PeerConnection.
    if let Some(ref existing) = stream {
      existing.add_track(&video_track);
      // Drop the synthetic video-only stream now that its track has
      // been adopted; the track itself stays alive because it has a
      // second owner (`existing`).
    } else {
      // No prior stream (e.g. media acquisition failed initially):
      // promote the video-only stream to be the preview source.
      self.signals.local_stream.set(Some(new_video_stream));
    }

    // H3 fix: keep `CallState.media_type` aligned with the actual
    // mode of the call so refresh-recovery, telemetry, and any other
    // observer of the state machine sees Video — not Audio — once
    // the camera has been added.
    if let CallState::Active {
      room_id,
      media_type: MediaType::Audio,
      started_at_ms,
    } = self.signals.call_state.get_untracked()
    {
      self.transition(CallState::Active {
        room_id,
        media_type: MediaType::Video,
        started_at_ms,
      });
      self.persist();
    }

    self.broadcast_media_state();
    Ok(new_enabled)
  }

  /// Start or stop screen-sharing. Toggles to the opposite of the
  /// current state; on switch-on this prompts the user via
  /// `getDisplayMedia` and publishes the resulting stream to every
  /// connected peer (P0 Bug-3 fix). When the user stops the share via
  /// the browser's system dialog, the `MediaStreamTrack.onended` hook
  /// installed below transparently switches back to the camera+mic.
  ///
  /// A `screen_share_switching` guard flag (P2-4 fix) prevents
  /// re-entrant calls — e.g. when the `onended` callback fires while
  /// a manual toggle is already in progress.
  ///
  /// Race semantics (Round-4 documentation):
  /// * The guard flag is a `Cell<bool>` owned by `Inner`, so there is
  ///   no cross-thread concern on the single-threaded WASM runtime.
  /// * The `Closure::once_into_js` handler attached below owns a
  ///   cloned `CallManager` but is dropped by the browser after its
  ///   single invocation (the `stop_stream` call in the turn-off
  ///   branch forces that invocation, so the closure never outlives
  ///   the call).
  /// * If an `onended` invocation arrives while a manual toggle is
  ///   still awaiting inside [`Self::toggle_screen_share_inner`], the
  ///   guard bounces the second call and the caller observes `Ok(())`.
  ///
  /// # Errors
  /// Returns `Err` if the display picker is cancelled or denied. On
  /// failure the previous local-media state is restored so the UI
  /// does not get stuck (P1 Bug-3 fix).
  pub async fn toggle_screen_share(&self) -> Result<(), String> {
    // P2-4 fix: bail out if a previous toggle is still in flight.
    if self.inner.borrow().screen_share_switching.get() {
      return Ok(());
    }
    self.inner.borrow().screen_share_switching.set(true);
    let result = self.toggle_screen_share_inner().await;
    self.inner.borrow().screen_share_switching.set(false);
    result
  }

  /// Inner implementation of screen-share toggle, called after the
  /// re-entrancy guard is acquired.
  async fn toggle_screen_share_inner(&self) -> Result<(), String> {
    let currently_on = self.signals.local_media.get_untracked().screen_sharing;

    if currently_on {
      // M5 fix: detach the `onended` handler before stopping the
      // track. Calling `stop()` triggers `onended` synchronously on
      // some browsers; without this clean-up that would re-enter
      // `toggle_screen_share` (bounced by the guard, but still a wasted
      // round-trip) and the boxed `Closure` would also linger on the
      // track until the next GC cycle.
      if let Some(stream) = self.signals.local_stream.get_untracked() {
        if let Some(track) = media::first_video_track(&stream) {
          track.set_onended(None);
        }
        media::stop_stream(&stream);
      }
      self.signals.local_stream.set(None);
      self
        .signals
        .local_media
        .update(|s| s.screen_sharing = false);

      let media_type = self
        .signals
        .call_state
        .get_untracked()
        .media_type()
        .unwrap_or(MediaType::Audio);
      match media::acquire_user_media(media_type).await {
        Ok(fresh) => {
          self.install_local_stream(media_type, fresh);
          self.broadcast_media_state();
          Ok(())
        }
        Err(e) => {
          // When stopping screen-share we already stopped the old
          // stream and cleared `local_stream`. If acquiring the
          // camera/mic fails here the user would be left with no
          // local media and a frozen last frame on the remote side.
          // Degrade to an explicit "off" state so the UI renders a
          // muted/no-camera placeholder and unpublish so the remote
          // tile falls back to the avatar (P1 Bug-3 fix).
          self.signals.local_media.set(LocalMediaState::off());
          if let Some(webrtc) = self.webrtc.borrow().as_ref() {
            webrtc.unpublish_local_media();
          }
          self.broadcast_media_state();
          Err(e)
        }
      }
    } else {
      let display = media::acquire_display_stream().await?;
      // Hook the system "Stop sharing" button so the screen-share
      // toggle stays in sync with the actual track state. We use a
      // weak-style clone to avoid retaining the manager via the JS
      // closure beyond the track's lifetime.
      if let Some(track) = media::first_video_track(&display) {
        let manager = self.clone();
        let on_ended = wasm_bindgen::closure::Closure::once_into_js(move || {
          // Best effort: kick off the toggle without blocking. If the
          // user manually toggled off in the meantime this is a no-op
          // because `currently_on` will already be false.
          if manager.signals.local_media.get_untracked().screen_sharing {
            let manager = manager.clone();
            spawn_local(async move {
              if let Err(e) = manager.toggle_screen_share().await {
                web_sys::console::warn_1(
                  &format!("[call] Auto-restore after screen-share end failed: {e}").into(),
                );
              }
            });
          }
        });
        // SAFETY: `Closure::once_into_js` returns a `JsValue`; cast
        // back to a `Function` for `set_onended`.
        use wasm_bindgen::JsCast;
        if let Ok(func) = on_ended.dyn_into::<js_sys::Function>() {
          track.set_onended(Some(&func));
        }
      }

      // Replace the local preview before publishing so the UI updates
      // synchronously even if `publish_local_stream` happens to log a
      // warning.
      if let Some(old) = self.signals.local_stream.get_untracked() {
        media::stop_stream(&old);
      }
      self.signals.local_stream.set(Some(display.clone()));
      self.signals.local_media.update(|s| s.screen_sharing = true);

      // P0 Bug-3 fix: actually publish the screen capture to every
      // connected peer. Without this the remote side keeps seeing the
      // previous camera frames (or nothing).
      if let Some(webrtc) = self.webrtc.borrow().as_ref() {
        webrtc.publish_local_stream(&display);
      }
      self.broadcast_media_state();
      Ok(())
    }
  }

  /// Request Picture-in-Picture for the given `<video>` element.
  ///
  /// # Errors
  /// Returns `Err` if the browser rejects the request.
  pub async fn enter_pip(&self, video: &web_sys::HtmlVideoElement) -> Result<(), String> {
    media::request_picture_in_picture(video).await?;
    self.signals.pip_active.set(true);
    Ok(())
  }

  /// Exit Picture-in-Picture if it is currently active.
  ///
  /// # Errors
  /// Returns `Err` if the browser rejects the call.
  pub async fn exit_pip(&self) -> Result<(), String> {
    media::exit_picture_in_picture().await?;
    self.signals.pip_active.set(false);
    Ok(())
  }

  /// Apply a new [`VideoProfile`] to the outgoing video track (used by
  /// the network-quality downgrader).
  ///
  /// # Errors
  /// Returns `Err` if `applyConstraints` rejects.
  pub async fn apply_video_profile(&self, profile: VideoProfile) -> Result<(), String> {
    let stream = self
      .signals
      .local_stream
      .get_untracked()
      .ok_or("No local stream to re-target")?;
    let Some(track) = media::first_video_track(&stream) else {
      return Ok(());
    };
    media::retarget_video_track(&track, profile).await
  }

  // ── Internal media helpers ─────────────────────────────────────

  /// Install the local capture stream **and** publish it to every peer
  /// (Active-call path).
  ///
  /// Splits into [`Self::prepare_local_stream`] (preview only — used
  /// while the call is still `Inviting`) and the publish step. The
  /// publish step is deferred until the remote side accepts (P1-New-3
  /// fix); sending `addTrack` earlier would pollute the mesh SDP for
  /// peers who never ultimately participate in the call.
  pub(super) fn install_local_stream(&self, media_type: MediaType, stream: MediaStream) {
    self.prepare_local_stream(media_type, stream.clone());
    self.publish_to_peers(&stream);
  }

  /// Install the local capture stream as the UI preview only, without
  /// pushing any track to peer connections. Used by the `Inviting`
  /// path so the caller sees their own preview while waiting for the
  /// callee to accept (P1-New-3 fix).
  pub(super) fn prepare_local_stream(&self, media_type: MediaType, stream: MediaStream) {
    self
      .signals
      .local_media
      .set(LocalMediaState::initial_for(media_type));
    self.signals.local_stream.set(Some(stream));
    // Reset the video profile to the baseline when a fresh stream is
    // installed (P2-New-6).
    self.signals.self_video_profile.set(VideoProfile::HIGH);
  }

  /// Publish the currently-installed local stream to every connected
  /// peer. Called from the `Active`-transition path (P1-New-3 fix).
  ///
  /// The WebRTC manager iterates its mesh snapshot internally; per-peer
  /// failures are logged but do not abort the call.
  pub(super) fn publish_to_peers(&self, stream: &MediaStream) {
    if let Some(webrtc) = self.webrtc.borrow().as_ref() {
      webrtc.publish_local_stream(stream);
    }
  }

  pub(super) fn tear_down_local_media(&self) {
    // Detach published senders before stopping tracks so the browser
    // release indicator clears promptly on the remote side.
    if let Some(webrtc) = self.webrtc.borrow().as_ref() {
      webrtc.unpublish_local_media();
    }
    if let Some(stream) = self.signals.local_stream.get_untracked() {
      media::stop_stream(&stream);
    }
    self.signals.local_stream.set(None);
    self.signals.local_media.set(LocalMediaState::off());
    self.signals.participants.update(HashMap::clear);
    self.signals.duration_secs.set(0);
    self.signals.pip_active.set(false);
    // Drop every VAD detector so the associated `AudioContext`s are
    // released to the browser.
    self.inner.borrow_mut().vad.clear();
  }

  /// Broadcast the local media state to every connected peer so remote
  /// UIs can render muted / camera-off / screen-sharing icons (Req 3.5 /
  /// 7.1 — DataChannel `MediaStateUpdate` message).
  pub(super) fn broadcast_media_state(&self) {
    let state = self.signals.local_media.get_untracked();
    let webrtc = self.webrtc.borrow().clone();
    let Some(webrtc) = webrtc else {
      return;
    };
    let update = message::datachannel::MediaStateUpdate {
      mic_enabled: state.mic_enabled,
      camera_enabled: state.camera_enabled,
      screen_sharing: state.screen_sharing,
    };
    let msg = message::datachannel::DataChannelMessage::MediaStateUpdate(update);
    webrtc.broadcast_data_channel_message(&msg);
  }

  /// If the current call state is `Active`, emit the duration summary
  /// toast (Req 7.5). Called from [`Self::transition`] immediately
  /// before the state flips to `Ended`.
  ///
  /// The toast carries only the formatted duration string as its
  /// fallback payload; the localised template
  /// `call.duration_summary = "Call duration: {{duration}}"` is
  /// resolved by the toast component, which substitutes the value via
  /// `apply_template_value`. This keeps the user-visible prefix in the
  /// i18n catalog instead of hard-coding English in Rust.
  pub(super) fn maybe_emit_duration_toast(&self) {
    let Some(started) = self
      .signals
      .call_state
      .get_untracked()
      .active_started_at_ms()
    else {
      return;
    };
    let elapsed_secs = ((now_ms().saturating_sub(started)).max(0) / 1000) as u64;
    if elapsed_secs == 0 {
      return;
    }
    let Some(toast) = self.error_toast.get() else {
      return;
    };
    let formatted = format_duration(elapsed_secs);
    // Use AV201 (a defined audio/video status code) rather than the
    // previously-used "AV500" which falls outside the AV001..=AV405
    // range allocated by the error-code spec.
    toast.show_info_message_with_key("AV201", "call.duration_summary", &formatted);
  }
}
