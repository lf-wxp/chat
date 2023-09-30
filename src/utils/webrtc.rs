use gloo_console::log;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{HtmlMediaElement, MediaStream, MediaStreamTrack, RtcPeerConnection};

use super::get_media;

pub struct WebRTC {
  peer_connection: RtcPeerConnection,
  dom: HtmlMediaElement,
  stream: MediaStream,
}

impl WebRTC {
  pub async fn new(video_dom: HtmlMediaElement) -> Result<Self, JsValue> {
    let peer = RtcPeerConnection::new()?;
    let stream = get_media(
      Some("{ device_id: 'default',echo_cancellation: true }"),
      Some("{ device_id: 'default' }"),
    )
    .await?;

    let webrtc = WebRTC {
      peer_connection: peer,
      dom: video_dom,
      stream,
    };
    webrtc.set_dom_stream();
    Ok(webrtc)
  }

  fn set_dom_stream(&self) {
    if let Some(stream) = self.dom.src_object() {
      log!("inside dom", &stream);
      stream
        .get_tracks()
        .iter()
        .filter_map(|track| track.dyn_into::<MediaStreamTrack>().ok())
        .for_each(|media_stream_track| {
          stream.remove_track(&media_stream_track);
        });
    }
    log!("dom", &self.dom, &self.stream);
    self.dom.set_src_object(Some(&self.stream));
  }
}
