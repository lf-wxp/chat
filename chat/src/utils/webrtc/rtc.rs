use async_broadcast::{broadcast, Receiver, Sender};
use js_sys::{Array, Reflect};
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
  MediaStream, MessageEvent, RtcDataChannel, RtcDataChannelEvent, RtcIceCandidate,
  RtcIceConnectionState, RtcPeerConnection, RtcPeerConnectionIceEvent, RtcSdpType,
  RtcSessionDescriptionInit, RtcTrackEvent,
};

use crate::{bind_event, model::IceCandidate};

pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug)]
pub struct WebRTC {
  peer: RtcPeerConnection,
  data_channel: RtcDataChannel,
  pub message_receiver: Receiver<ChannelMessage>,
  pub message_sender: Sender<ChannelMessage>,
}

#[derive(Debug, Clone)]
pub enum ChannelMessage {
  ErrorEvent,
  TrackEvent(RtcTrackEvent),
  DataChannelEvent(RtcDataChannelEvent),
  IceEvent(RtcPeerConnectionIceEvent),
  DataChannelCloseEvent,
  DataChannelErrorEvent,
  DataChannelMessage(MessageEvent),
}

impl WebRTC {
  pub fn new() -> Result<Self, JsValue> {
    let (sender, receiver) = broadcast(20);
    let peer = RtcPeerConnection::new()?;
    let data_channel = peer.create_data_channel("chat");
    let rtc = Self {
      peer,
      data_channel,
      message_receiver: receiver,
      message_sender: sender,
    };
    rtc.setup();
    Ok(rtc)
  }

  fn setup(&self) {
    self.bind_ontrack();
    self.bind_ondatachannel();
    self.bind_onicecandidate();
    self.bind_ondatachannel_message();
  }

  fn bind_ontrack(&self) {
    bind_event!(
      self.peer,
      "track",
      self.message_sender,
      ChannelMessage::TrackEvent,
      RtcTrackEvent
    )
  }

  fn bind_ondatachannel(&self) {
    bind_event!(
      self.peer,
      "datachannel",
      self.message_sender,
      ChannelMessage::DataChannelEvent,
      RtcDataChannelEvent
    )
  }

  fn bind_onicecandidate(&self) {
    bind_event!(
      self.peer,
      "icecandidate",
      self.message_sender,
      ChannelMessage::IceEvent,
      RtcPeerConnectionIceEvent
    )
  }

  fn bind_ondatachannel_message(&self) {
    bind_event!(
      self.data_channel,
      "message",
      self.message_sender,
      ChannelMessage::DataChannelMessage,
      MessageEvent
    )
  }

  pub fn state(&self) -> RtcIceConnectionState {
    self.peer.ice_connection_state()
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

  pub async fn get_send_offer(&self) -> Result<String, JsValue> {
    let offer = JsFuture::from(self.peer.create_offer()).await?;
    let offer_sdp = Reflect::get(&offer, &JsValue::from_str("sdp"))?
      .as_string()
      .unwrap();
    let offer_obj = WebRTC::create_offer(&offer_sdp);
    JsFuture::from(self.peer.set_local_description(&offer_obj)).await?;
    Ok(offer_sdp)
  }

  pub async fn receive_offer(&self, sdp: String) -> Result<(), JsValue> {
    let offer_obj = WebRTC::create_offer(&sdp);
    JsFuture::from(self.peer.set_remote_description(&offer_obj)).await?;
    Ok(())
  }

  pub async fn get_send_answer(&self) -> Result<String, JsValue> {
    let answer = JsFuture::from(self.peer.create_answer()).await?;
    let answer_sdp = Reflect::get(&answer, &JsValue::from_str("sdp"))?
      .as_string()
      .unwrap();
    let answer_obj = WebRTC::create_answer(&answer_sdp);
    JsFuture::from(self.peer.set_local_description(&answer_obj)).await?;
    Ok(answer_sdp)
  }
  pub fn set_tracks(&self, stream: MediaStream) {
    let audio_tracks = stream.get_audio_tracks();
    for i in 0..audio_tracks.length() {
      let track = audio_tracks
        .get(i)
        .dyn_into::<web_sys::MediaStreamTrack>()
        .unwrap();
      let more_streams = Array::new();
      self.peer.add_track(&track, &stream, &more_streams);
    }

    let video_tracks = stream.get_video_tracks();
    for i in 0..video_tracks.length() {
      let track = video_tracks
        .get(i)
        .dyn_into::<web_sys::MediaStreamTrack>()
        .unwrap();
      let more_streams = Array::new();
      self.peer.add_track(&track, &stream, &more_streams);
    }
  }

  pub async fn receive_answer(&self, sdp: String) -> Result<(), JsValue> {
    let answer_obj = WebRTC::create_answer(&sdp);
    JsFuture::from(self.peer.set_remote_description(&answer_obj)).await?;
    Ok(())
  }

  pub fn receive_ice(&self, ice_candidate_json: String) -> Result<(), JsValue> {
    let ice_candidate = serde_json::from_str::<IceCandidate>(&ice_candidate_json).unwrap();
    let ice = RtcIceCandidate::try_from(ice_candidate.clone()).ok();
    let _ = self
      .peer
      .add_ice_candidate_with_opt_rtc_ice_candidate(ice.as_ref());
    Ok(())
  }
}
