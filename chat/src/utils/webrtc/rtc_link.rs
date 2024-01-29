use gloo_console::log;
use js_sys::{Array, Reflect, JSON};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
  HtmlMediaElement, MediaStream, RtcDataChannel, RtcIceCandidate,
  RtcIceConnectionState, RtcPeerConnection, RtcPeerConnectionIceEvent, RtcSdpType,
  RtcSessionDescriptionInit, RtcTrackEvent,
};

use crate::{model::IceCandidate, utils::{query_selector, get_user_media}};

pub struct RTCLink {
  peer_id: String,
  peer: RtcPeerConnection,
  data_channel: RtcDataChannel,
  remote_media: MediaStream,
}

impl RTCLink {
  pub fn new(peer_id: String) -> Result<Self, JsValue> {
    let peer = RtcPeerConnection::new()?;
    let data_channel = peer.create_data_channel("chat");
    let link = RTCLink {
      peer_id,
      peer,
      data_channel,
      remote_media: MediaStream::new()?,
    };
    Ok(link)
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

  fn ontrack(&self, ev: RtcTrackEvent) {
    log!("ontrack", ev.track());
    self.remote_media.add_track(&ev.track());
    if let Some(dom) = query_selector::<HtmlMediaElement>(".remote-stream") {
      dom.set_src_object(Some(self.remote_media.as_ref()));
    }
  }

  fn onicecandidate(&self, ev: RtcPeerConnectionIceEvent) -> Option<String> {
    ev.candidate().map(|candidate| {
      let json_candidate = JSON::stringify(&candidate.to_json())
        .unwrap()
        .as_string()
        .unwrap();
      log!("onicecandidate", &json_candidate);
      json_candidate
    })
  }

  pub fn state(&self) -> RtcIceConnectionState {
    self.peer.ice_connection_state()
  }

  pub async fn get_send_offer(&self) -> Result<String, JsValue> {
    let offer = JsFuture::from(self.peer.create_offer()).await?;
    let offer_sdp = Reflect::get(&offer, &JsValue::from_str("sdp"))?
      .as_string()
      .unwrap();
    let offer_obj = RTCLink::create_offer(&offer_sdp);
    JsFuture::from(self.peer.set_local_description(&offer_obj)).await?;
    log!("send offer");
    Ok(offer_sdp)
  }

  pub async fn receive_offer(&self, sdp: String) -> Result<(), JsValue> {
    let offer_obj = RTCLink::create_offer(&sdp);
    JsFuture::from(self.peer.set_remote_description(&offer_obj)).await?;
    log!("receive offer inner");
    Ok(())
  }

  pub async fn get_send_answer(&self) -> Result<String, JsValue> {
    let answer = JsFuture::from(self.peer.create_answer()).await?;
    let answer_sdp = Reflect::get(&answer, &JsValue::from_str("sdp"))?
      .as_string()
      .unwrap();
    let answer_obj = RTCLink::create_answer(&answer_sdp);
    JsFuture::from(self.peer.set_local_description(&answer_obj)).await?;
    log!("send answer");
    Ok(answer_sdp)
  }

  pub async fn receive_answer(&self, sdp: String) -> Result<(), JsValue> {
    let answer_obj = RTCLink::create_answer(&sdp);
    JsFuture::from(self.peer.set_remote_description(&answer_obj)).await?;
    log!("receive answer");
    Ok(())
  }

  pub fn receive_ice(&self, ice_candidate_json: String) -> Result<(), JsValue> {
    let ice_candidate = serde_json::from_str::<IceCandidate>(&ice_candidate_json).unwrap();
    let ice = RtcIceCandidate::try_from(ice_candidate.clone()).ok();
    log!(
      "add ice candidate",
      format!("{:?}----{:?}", &ice, &ice_candidate),
      ice_candidate_json
    );
    let _ = self
      .peer
      .add_ice_candidate_with_opt_rtc_ice_candidate(ice.as_ref());
    Ok(())
  }

  pub fn set_tracks(&self, stream: MediaStream) {
    let audio_tracks = stream.get_audio_tracks();
    for i in 0..audio_tracks.length() {
      let track = audio_tracks
        .get(i)
        .dyn_into::<web_sys::MediaStreamTrack>()
        .unwrap();
      let more_streams = Array::new();
      log!("set tracks audio");
      self.peer.add_track(&track, &stream, &more_streams);
    }

    let video_tracks = stream.get_video_tracks();
    for i in 0..video_tracks.length() {
      let track = video_tracks
        .get(i)
        .dyn_into::<web_sys::MediaStreamTrack>()
        .unwrap();
      log!("set tracks video");
      let more_streams = Array::new();
      self.peer.add_track(&track, &stream, &more_streams);
    }
  }

  pub async fn set_local_user_media(&self, dom: Option<HtmlMediaElement>) -> Result<(), JsValue> {
    let stream = get_user_media(
      // Some("{ device_id: 'default',echo_cancellation: true }"),
      None,
      Some("true"),
    )
    .await
    .ok();
    if let Some(dom) = dom {
      log!("stream", &dom);
      dom.set_src_object(stream.as_ref());
    }
    if let Some(stream) = stream {
      self.set_tracks(stream);
    }
    Ok(())
  }
}
