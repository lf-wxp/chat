use gloo_console::log;
use js_sys::{Array, Reflect};
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
  HtmlMediaElement, MediaStream, MediaStreamTrack, RtcDataChannel, RtcDataChannelEvent,
  RtcPeerConnection, RtcPeerConnectionIceEvent, RtcSdpType, RtcSessionDescriptionInit,
  RtcTrackEvent,
};

use crate::utils::set_dom_stream;

use super::get_media;

pub struct WebRTC {
  peer_connection: RtcPeerConnection,
  pub stream: Option<MediaStream>,
  sdp: Option<String>,
  channel: RtcDataChannel,
  sdp_obj: Option<RtcSessionDescriptionInit>,
  remote_sdp: Option<String>,
  remote_stream: MediaStream,
}

impl WebRTC {
  pub fn new() -> Result<Self, JsValue> {
    let peer = RtcPeerConnection::new()?;
    let channel = peer.create_data_channel("chat");
    let remote_stream = MediaStream::new()?;
    let rtc = WebRTC {
      peer_connection: peer,
      stream: None,
      sdp: None,
      channel,
      sdp_obj: None,
      remote_sdp: None,
      remote_stream,
    };
    rtc.bind_webrtc_event();
    Ok(rtc)
  }

  fn bind_webrtc_event(&self) {
    self.bind_ontrack();
    self.bind_ondatachannel();
    self.bind_onicecandidate();
    self.bind_onnegotiationneeded();
  }

  fn bind_ontrack(&self) {
    let peer = self.peer_connection.clone();
    let remote_stream = self.remote_stream.clone();
    let ontrack_callback = Closure::<dyn FnMut(_)>::new(move |ev: RtcTrackEvent| {
      log!("ontrack", ev.track());
      remote_stream.add_track(&ev.track());
      set_dom_stream(".remote-stream", Some(&remote_stream));
    });
    self
      .peer_connection
      .set_ontrack(Some(ontrack_callback.as_ref().unchecked_ref()));
    ontrack_callback.forget();
  }

  fn bind_ondatachannel(&self) {
    let peer = self.peer_connection.clone();
    let ondatachanel_callback = Closure::<dyn FnMut(_)>::new(move |ev: RtcDataChannelEvent| {
      log!("ondatachanel", ev);
    });
    self
      .peer_connection
      .set_ondatachannel(Some(ondatachanel_callback.as_ref().unchecked_ref()));
    ondatachanel_callback.forget();
  }

  fn bind_onicecandidate(&self) {
    let peer = self.peer_connection.clone();
    let onicecandidate_callback =
      Closure::<dyn FnMut(_)>::new(move |ev: RtcPeerConnectionIceEvent| {
        if let Some(candidate) = ev.candidate() {
          log!("onicecandidate", ev.candidate());
          let _ = peer.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&candidate));
        }
      });
    self
      .peer_connection
      .set_onicecandidate(Some(onicecandidate_callback.as_ref().unchecked_ref()));
    onicecandidate_callback.forget();
  }

  fn bind_onnegotiationneeded(&self) {
    let onnegotiationneeded_callback =
      Closure::<dyn FnMut(_)>::new(move |_: RtcDataChannelEvent| {
        log!("onnegotiationneeded");
      });
    self
      .peer_connection
      .set_onnegotiationneeded(Some(onnegotiationneeded_callback.as_ref().unchecked_ref()));
    onnegotiationneeded_callback.forget();
  }

  pub async fn set_stream(&mut self) -> Result<(), JsValue> {
    if self.stream.is_some() {
      return Ok(());
    }
    let stream = get_media(
      Some("{ device_id: 'default',echo_cancellation: true }"),
      Some("{ device_id: 'default' }"),
    )
    .await?;
    self.stream = Some(stream);
    Ok(())
  }

  pub fn set_local_stream_dom(&self, dom: HtmlMediaElement) {
    if let Some(stream) = dom.src_object() {
      log!("local dom", &stream);
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

  pub fn set_remote_stream_dom(&self, dom: HtmlMediaElement) {
    if let Some(stream) = dom.src_object() {
      log!("remote dom", &stream);
      stream
        .get_tracks()
        .iter()
        .filter_map(|track| track.dyn_into::<MediaStreamTrack>().ok())
        .for_each(|media_stream_track| {
          stream.remove_track(&media_stream_track);
        });
    }
    dom.set_src_object(Some(self.remote_stream.as_ref()));
  }

  pub async fn sdp(&mut self) -> Option<String> {
    if self.sdp.is_some() {
      return self.sdp.clone();
    }
    self.emit_offer().await.ok()
  }

  pub fn attach_stream(&self) -> Result<(), JsValue> {
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

  async fn emit_offer(&mut self) -> Result<String, JsValue> {
    if self.sdp.is_some() {
      return self.sdp.clone().ok_or(JsValue::from_str("error"));
    }
    let offer = JsFuture::from(self.peer_connection.create_offer()).await?;
    let offer_sdp = Reflect::get(&offer, &JsValue::from_str("sdp"))?
      .as_string()
      .unwrap();
    let offer_obj = WebRTC::create_offer(&offer_sdp);
    JsFuture::from(self.peer_connection.set_local_description(&offer_obj)).await?;
    self.sdp = Some(offer_sdp.clone());
    Ok(offer_sdp)
  }

  pub async fn receive_and_emit_offer(&mut self, sdp: String) -> Result<String, JsValue> {
    let offer_obj = WebRTC::create_offer(&sdp);
    JsFuture::from(self.peer_connection.set_remote_description(&offer_obj)).await?;
    let answer = JsFuture::from(self.peer_connection.create_answer()).await?;
    let answer_sdp = Reflect::get(&answer, &JsValue::from_str("sdp"))?
      .as_string()
      .unwrap();
    self.sdp = Some(answer_sdp.clone());
    let answer_obj = WebRTC::create_answer(&answer_sdp);
    JsFuture::from(self.peer_connection.set_local_description(&answer_obj)).await?;
    log!("answer", &self.peer_connection);
    Ok(answer_sdp)
  }

  pub async fn receive_answer(&mut self, sdp: String) -> Result<(), JsValue> {
    let answer_obj = WebRTC::create_answer(&sdp);
    JsFuture::from(self.peer_connection.set_remote_description(&answer_obj)).await?;
    Ok(())
  }
}
