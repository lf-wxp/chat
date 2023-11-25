use std::{cell::RefCell, rc::Rc};

use gloo_console::log;
use js_sys::{Array, Reflect, JSON};
use message::Signal;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{
  HtmlMediaElement, MediaStream, RtcDataChannel, RtcDataChannelEvent, RtcIceCandidate,
  RtcIceCandidateInit, RtcPeerConnection, RtcPeerConnectionIceEvent, RtcSdpType,
  RtcSessionDescriptionInit, RtcTrackEvent,
};

use crate::{model::IceCandidate, utils::query_selector};

use super::get_user_media;

pub struct RTCLink {
  peer: RtcPeerConnection,
  data_channel: RtcDataChannel,
  signal_channel: Rc<RefCell<dyn Signal>>,
  this: Option<Rc<RefCell<Self>>>,
  remote_media: Rc<RefCell<MediaStream>>,
}

impl RTCLink {
  pub fn new(signal_channel: Rc<RefCell<dyn Signal>>) -> Result<Rc<RefCell<Self>>, JsValue> {
    let peer = RtcPeerConnection::new()?;
    let data_channel = peer.create_data_channel("chat");
    let link = Rc::new(RefCell::new(RTCLink {
      peer,
      data_channel,
      signal_channel,
      this: None,
      remote_media: Rc::new(RefCell::new(MediaStream::new().unwrap())),
    }));
    link.borrow_mut().this = Some(link.clone());
    link.borrow_mut().bind_signal_event();
    link.borrow_mut().bind_webrtc_event();
    Ok(link)
  }

  fn bind_signal_event(&self) {
    if let Some(link) = self.this.clone() {
      let link_clone = link.clone();
      self
        .signal_channel
        .borrow_mut()
        .set_receive_answer(Box::new(move |sdp| {
          let link_clone = link_clone.clone();
          spawn_local(async move {
            let _ = link_clone.borrow().receive_answer(sdp).await;
          });
        }));
      let link_clone = link.clone();
      self
        .signal_channel
        .borrow_mut()
        .set_receive_offer(Box::new(move |sdp| {
          log!("receive offer");
          let link_clone = link_clone.clone();
          spawn_local(async move {
            let _ = link_clone.borrow().receive_offer(sdp).await;
            let _ = link_clone.borrow().send_answer().await;
          });
        }));
      let link_clone = link.clone();
      self
        .signal_channel
        .borrow_mut()
        .set_receive_ice(Box::new(move |ice| {
          link_clone.borrow().receive_ice(ice);
        }));
    }
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

  fn bind_webrtc_event(&self) {
    self.bind_ontrack();
    self.bind_ondatachannel();
    self.bind_onicecandidate();
    self.bind_onnegotiationneeded();
    self.bind_onconnectionstatechange();
  }

  fn bind_ontrack(&self) {
    let peer = self.peer.clone();
    let media = self.remote_media.clone();
    let ontrack_callback = Closure::<dyn FnMut(_)>::new(move |ev: RtcTrackEvent| {
      log!("ontrack", ev.track());
      media.borrow_mut().add_track(&ev.track());
      if let Some(dom) = query_selector::<HtmlMediaElement>(".remote-stream") {
        dom.set_src_object(Some(media.borrow().as_ref()));
      }
    });
    self
      .peer
      .set_ontrack(Some(ontrack_callback.as_ref().unchecked_ref()));
    ontrack_callback.forget();
  }

  fn bind_ondatachannel(&self) {
    let peer = self.peer.clone();
    let ondatachanel_callback = Closure::<dyn FnMut(_)>::new(move |ev: RtcDataChannelEvent| {
      log!("ondatachanel", ev);
    });
    self
      .peer
      .set_ondatachannel(Some(ondatachanel_callback.as_ref().unchecked_ref()));
    ondatachanel_callback.forget();
  }

  fn bind_onicecandidate(&self) {
    let channel = self.signal_channel.clone();
    let onicecandidate_callback =
      Closure::<dyn FnMut(_)>::new(move |ev: RtcPeerConnectionIceEvent| {
        if let Some(candidate) = ev.candidate() {
          let json_candidate = JSON::stringify(&candidate.to_json())
            .unwrap()
            .as_string()
            .unwrap();
          log!("onicecandidate", &json_candidate);
          channel.borrow_mut().send_ice(json_candidate);
        }
      });
    self
      .peer
      .set_onicecandidate(Some(onicecandidate_callback.as_ref().unchecked_ref()));
    onicecandidate_callback.forget();
  }

  fn bind_onnegotiationneeded(&self) {
    let onnegotiationneeded_callback =
      Closure::<dyn FnMut(_)>::new(move |_: RtcDataChannelEvent| {
        log!("onnegotiationneeded");
      });
    self
      .peer
      .set_onnegotiationneeded(Some(onnegotiationneeded_callback.as_ref().unchecked_ref()));
    onnegotiationneeded_callback.forget();
  }

  fn bind_onconnectionstatechange(&self) {
    let peer = self.peer.clone();
    let onconnectionstatechange_callback =
      Closure::<dyn FnMut(_)>::new(move |_: RtcPeerConnection| {
        log!("onconnectionstatechange", peer.ice_connection_state());
      });
    self.peer.set_onconnectionstatechange(Some(
      onconnectionstatechange_callback.as_ref().unchecked_ref(),
    ));
    onconnectionstatechange_callback.forget();
  }

  pub async fn send_offer(&self) -> Result<(), JsValue> {
    let offer = JsFuture::from(self.peer.create_offer()).await?;
    let offer_sdp = Reflect::get(&offer, &JsValue::from_str("sdp"))?
      .as_string()
      .unwrap();
    let offer_obj = RTCLink::create_offer(&offer_sdp);
    JsFuture::from(self.peer.set_local_description(&offer_obj)).await?;
    self.signal_channel.borrow_mut().send_offer(offer_sdp);
    log!("send offer");
    Ok(())
  }

  pub async fn receive_offer(&self, sdp: String) -> Result<(), JsValue> {
    let offer_obj = RTCLink::create_offer(&sdp);
    JsFuture::from(self.peer.set_remote_description(&offer_obj)).await?;
    log!("receive offer inner");
    Ok(())
  }

  pub async fn send_answer(&self) -> Result<(), JsValue> {
    let answer = JsFuture::from(self.peer.create_answer()).await?;
    let answer_sdp = Reflect::get(&answer, &JsValue::from_str("sdp"))?
      .as_string()
      .unwrap();
    let answer_obj = RTCLink::create_answer(&answer_sdp);
    JsFuture::from(self.peer.set_local_description(&answer_obj)).await?;
    self.signal_channel.borrow_mut().send_answer(answer_sdp);
    log!("send answer");
    Ok(())
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
