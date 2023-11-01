use gloo_console::log;
use js_sys::{Array, Reflect};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
  HtmlMediaElement, MediaStream, MediaStreamTrack, RtcPeerConnection, RtcSdpType,
  RtcSessionDescriptionInit,
};

use super::get_media;

#[derive(Debug, Clone)]
pub struct WebRTC {
  peer_connection: RtcPeerConnection,
  stream: Option<MediaStream>,
  sdp: Option<String>,
  sdp_obj: Option<RtcSessionDescriptionInit>,
  remote_sdp: Option<String>,
}

impl WebRTC {
  pub fn new() -> Result<Self, JsValue> {
    let peer = RtcPeerConnection::new()?;
    let rtc = WebRTC {
      peer_connection: peer,
      stream: None,
      sdp: None,
      sdp_obj: None,
      remote_sdp: None,
    };
    Ok(rtc)
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

  pub async fn sdp(&mut self) -> Option<String> {
    if self.sdp.is_some() {
      return self.sdp.clone();
    }
    self.set_offer().await.ok()
  }

  fn attach_stream(&self) -> Result<(), JsValue> {
    if let Some(stream) = &self.stream {
      let tracks = stream.get_tracks();
      let main_track = tracks.at(0).dyn_into::<MediaStreamTrack>()?;
      let more_track = tracks.slice(1, tracks.length() - 1);
      self
        .peer_connection
        .add_track(&main_track, stream, &more_track);
    }
    Ok(())
  }

  fn create_answer(sdp: &str) -> RtcSessionDescriptionInit {
    let mut answer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
    answer_obj.sdp(sdp);
    answer_obj
  }

  fn create_offer(sdp: &str) -> RtcSessionDescriptionInit {
    let mut offer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
    offer_obj.sdp(sdp);
    offer_obj
  }

  async fn set_offer(&mut self) -> Result<String, JsValue> {
    let offer = JsFuture::from(self.peer_connection.create_offer()).await?;
    let offer_sdp = Reflect::get(&offer, &JsValue::from_str("sdp"))?
      .as_string()
      .unwrap();
    let offer_obj = WebRTC::create_offer(&offer_sdp);
    JsFuture::from(self.peer_connection.set_local_description(&offer_obj)).await?;
    self.sdp = Some(offer_sdp.clone());
    Ok(offer_sdp)
  }

  async fn receive_answer(&mut self, sdp: String) -> Result<(), JsValue> {
    let answer_obj = WebRTC::create_answer(&sdp);
    JsFuture::from(self.peer_connection.set_remote_description(&answer_obj)).await?;
    Ok(())
  }

  async fn offer_answer(&self) -> Result<String, JsValue> {
    let answer = JsFuture::from(self.peer_connection.create_answer()).await?;
    let answer_sdp = Reflect::get(&answer, &JsValue::from_str("sdp"))?
      .as_string()
      .unwrap();
    Ok(answer_sdp)
  }
}
