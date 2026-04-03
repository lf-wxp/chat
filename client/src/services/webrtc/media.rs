//! Media stream and track management

use leptos::prelude::*;
use wasm_bindgen::JsCast;

use super::PeerManager;

impl PeerManager {
  /// Set enable/disable state of media tracks in specified connection
  ///
  /// `kind` is `"audio"` or `"video"`.
  pub fn set_track_enabled(&self, remote_user_id: &str, kind: &str, enabled: bool) {
    self.peers.with_value(|peers| {
      if let Some(entry) = peers.get(remote_user_id) {
        let senders = entry.connection.get_senders();
        for sender in senders.iter() {
          if let Ok(sender) = sender.dyn_into::<web_sys::RtcRtpSender>()
            && let Some(track) = sender.track()
            && track.kind() == kind
          {
            track.set_enabled(enabled);
          }
        }
      }
    });
  }

  /// Add local media stream to specified connection (audio/video call)
  pub fn add_media_stream(&self, remote_user_id: &str, stream: &web_sys::MediaStream) {
    self.peers.with_value(|peers| {
      if let Some(entry) = peers.get(remote_user_id) {
        for track in stream.get_tracks().iter() {
          if let Ok(track) = track.dyn_into::<web_sys::MediaStreamTrack>() {
            let streams = js_sys::Array::new();
            streams.push(stream);
            entry.connection.add_track(&track, stream, &streams);
          }
        }
      }
    });
  }

  /// Replace video track in specified connection (for screen sharing switch)
  ///
  /// Replace the currently sent video track with a new track without renegotiating SDP.
  pub fn replace_video_track(&self, remote_user_id: &str, new_track: &web_sys::MediaStreamTrack) {
    self.peers.with_value(|peers| {
      if let Some(entry) = peers.get(remote_user_id) {
        let senders = entry.connection.get_senders();
        for sender in senders.iter() {
          if let Ok(sender) = sender.dyn_into::<web_sys::RtcRtpSender>()
            && let Some(track) = sender.track()
            && track.kind() == "video"
          {
            let _ = sender.replace_track(Some(new_track));
            return;
          }
        }
      }
    });
  }

  /// Replace video tracks in all connections (broadcast screen sharing)
  pub fn replace_all_video_tracks(&self, new_track: &web_sys::MediaStreamTrack) {
    self.peers.with_value(|peers| {
      for entry in peers.values() {
        let senders = entry.connection.get_senders();
        for sender in senders.iter() {
          if let Ok(sender) = sender.dyn_into::<web_sys::RtcRtpSender>()
            && let Some(track) = sender.track()
            && track.kind() == "video"
          {
            let _ = sender.replace_track(Some(new_track));
          }
        }
      }
    });
  }
}
