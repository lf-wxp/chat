use gloo_console::log;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{HtmlMediaElement, MediaStream, MediaStreamTrack, RtcPeerConnection};

use super::get_media;

#[derive(Debug, Clone)]
pub struct WebRTC {
  peer_connection: RtcPeerConnection,
  stream: Option<MediaStream>,
  sdp: Option<String>,
  remote_sdp: Option<String>,
}

impl WebRTC {
  pub fn new() -> Result<Self, JsValue> {
    let peer = RtcPeerConnection::new()?;
    Ok(WebRTC {
      peer_connection: peer,
      stream: None,
      sdp: None,
      remote_sdp: None,
    })
  }

  pub async fn set_stream(&mut self) -> Result<(), JsValue> {
    let stream = get_media(
      Some("{ device_id: 'default',echo_cancellation: true }"),
      Some("{ device_id: 'default' }"),
    )
    .await?;
    self.stream = Some(stream);
    Ok(())
  }

  pub fn set_dom_stream(&self, dom: HtmlMediaElement) {
    if let Some(stream) = dom.src_object() {
      log!("inside dom", &stream);
      stream
        .get_tracks()
        .iter()
        .filter_map(|track| track.dyn_into::<MediaStreamTrack>().ok())
        .for_each(|media_stream_track| {
          stream.remove_track(&media_stream_track);
        });
    }
    dom.set_src_object(self.stream.as_ref());
  }
}
