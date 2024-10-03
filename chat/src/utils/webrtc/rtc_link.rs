use async_broadcast::{Receiver, Sender};
use gloo_console::log;
use js_sys::{ArrayBuffer, JSON};
use message::{
  CastMessage, ConnectMessage, ConnectState, MediaType, MessageType, RequestMessage,
  RequestMessageData, ResponseMessage, ResponseMessageData, SdpMessage, SdpType, SignalMessage,
};
use nanoid::nanoid;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlMediaElement, MediaStream, RtcIceConnectionState, RtcPeerConnection};
use yew::Event;

use crate::utils::{get_link, get_target, get_user_media, query_selector, to_connect_state, Link};

use super::{
  rtc::{TransmitMessage, WebRTC},
  Connect, ConnectError,
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RtcType {
  Caller,
  Callee,
}

#[derive(Debug, Clone)]
pub struct BaseInfo {
  id: String,
  remote_id: String,
  rtc: Rc<RefCell<WebRTC>>,
  rtc_type: RtcType,
}

#[derive(Debug)]
pub struct RTCLink {
  id: String,
  remote_id: String,
  remote_media: Rc<RefCell<MediaStream>>,
  ready: Rc<RefCell<bool>>,
  datachannel_ready: Rc<RefCell<bool>>,
  link: &'static mut Link,
  rtc: Rc<RefCell<WebRTC>>,
  rtc_type: RtcType,
  sender: Sender<ArrayBuffer>,
}

impl RTCLink {
  pub fn new(id: String, remote_id: String, sender: Sender<ArrayBuffer>, rtc_type: RtcType) -> Result<Self, JsValue> {
    let rtc = WebRTC::new()?;
    let link = get_link().unwrap();
    let remote_media = MediaStream::new()?;
    let mut link = RTCLink {
      id,
      remote_id,
      rtc: Rc::new(RefCell::new(rtc)),
      ready: Rc::new(RefCell::new(false)),
      datachannel_ready: Rc::new(RefCell::new(false)),
      remote_media: Rc::new(RefCell::new(remote_media)),
      link,
      rtc_type,
      sender,
    };
    link.watch_rtc_event();
    if link.rtc_type == RtcType::Caller {
      link.create_datachannel();
    }
    Ok(link)
  }

  fn get_base_info(&self) -> BaseInfo {
    let RTCLink {
      id,
      remote_id,
      rtc,
      rtc_type,
      ..
    } = self;
    BaseInfo {
      id: id.to_string(),
      remote_id: remote_id.to_string(),
      rtc: rtc.clone(),
      rtc_type: (*rtc_type).clone(),
    }
  }


  fn watch_rtc_event(&self) {
    let mut receiver_rtc = self.rtc.borrow().message_receiver.clone();
    let sender = self.link.sender();
    let receiver = self.link.receiver();
    let remote_media = self.remote_media.clone();
    let ready = self.ready.clone();
    let datachannel_ready = self.datachannel_ready.clone();
    let base_info = self.get_base_info();
    let channel_message_sender = self.sender.clone();
    spawn_local(async move {
      while let Ok(msg) = receiver_rtc.recv().await {
        match msg {
          TransmitMessage::ErrorEvent => {}
          TransmitMessage::TrackEvent(ev) => {
            log!("track event");
            remote_media.borrow_mut().add_track(&ev.track());
            if let Some(dom) = query_selector::<HtmlMediaElement>(".remote-stream") {
              dom.set_src_object(Some(&remote_media.borrow()));
            }
          }
          TransmitMessage::DataChannelEvent(ev) => {
            log!("data channel data");
          }
          TransmitMessage::IceEvent(ev) => {
            let ice = ev.candidate().map(|candidate| {
              JSON::stringify(&candidate.to_json())
                .unwrap()
                .as_string()
                .unwrap()
            });
            if ice.is_some() {
              let message = CastMessage::Ice(ice.unwrap());
              RTCLink::send_signal(&sender, base_info.clone(), message, nanoid!()).await;
            }
          }
          TransmitMessage::DataChannelCloseEvent => {}
          TransmitMessage::DataChannelErrorEvent => {}
          TransmitMessage::DataChannelMessage(ev) => {
            let _ = channel_message_sender.broadcast(ev.data().into()).await;
          }
          TransmitMessage::IceConnectionStateChange(ev) => {
            let target = get_target::<Event, RtcPeerConnection>(ev);
            let state = target.as_ref().unwrap().ice_connection_state();
            log!("ice change", format!("{:?}", &state));
            if target.is_some() {
              let is_connected = state == RtcIceConnectionState::Connected;
              *ready.borrow_mut() = is_connected;
              RTCLink::transmit_ice_state_message(
                &sender,
                base_info.clone(),
                to_connect_state(state),
                None,
              )
              .await;
            }
          }
          TransmitMessage::Negotiationneeded(ev) => {
            log!("track negotiation");
            let _ = RTCLink::negotiation(&sender, receiver.clone(), base_info.clone()).await;
          }
          TransmitMessage::DataChannelOpenEvent(ev) => {
            *datachannel_ready.borrow_mut() = true;
          }
        }
      }
    })
  }

  async fn send_signal(
    sender: &Sender<String>,
    base_info: BaseInfo,
    message: CastMessage,
    session_id: String,
  ) {
    let BaseInfo { id, remote_id, .. } = base_info;
    let message = serde_json::to_string(&RequestMessage {
      message: RequestMessageData::Signal(SignalMessage {
        from: id,
        to: remote_id,
        message,
      }),
      session_id,
      message_type: MessageType::Request,
    })
    .unwrap();
    let _ = sender.broadcast_direct(message).await;
  }

  async fn transmit_ice_state_message(
    sender: &Sender<String>,
    base_info: BaseInfo,
    state: ConnectState,
    media_type: Option<MediaType>,
  ) {
    let BaseInfo { id, remote_id, .. } = base_info;
    let message = serde_json::to_string(&RequestMessage {
      message: RequestMessageData::Connect(ConnectMessage {
        from: id,
        to: remote_id,
        state,
        media_type,
      }),
      session_id: nanoid!(),
      message_type: MessageType::Request,
    })
    .unwrap();
    let _ = sender.broadcast_direct(message).await;
  }

  pub fn is_ready(&self) -> bool {
    *self.ready.clone().borrow()
  }

  pub fn is_datachannel_ready(&self) -> bool {
    *self.datachannel_ready.clone().borrow()
  }

  pub async fn negotiation(
    sender: &Sender<String>,
    receiver: Receiver<String>,
    base_info: BaseInfo,
  ) -> Result<(), ConnectError> {
    if base_info.rtc_type == RtcType::Callee {
      return Ok(());
    }
    let remote_id = base_info.remote_id.clone();
    let send_future = RTCLink::send_offer(sender, base_info);
    let connect = Connect::new(remote_id, receiver);
    connect.connect(send_future).await
  }

  pub async fn send_offer(sender: &Sender<String>, base_info: BaseInfo) -> Result<(), JsValue> {
    let rtc = base_info.rtc.clone();
    let offer = rtc.borrow().get_send_offer().await?;
    let message = CastMessage::Sdp(SdpMessage {
      sdp_type: SdpType::Offer,
      sdp: offer,
    });
    RTCLink::send_signal(sender, base_info, message, nanoid!()).await;
    Ok(())
  }

  pub async fn send_answer(
    sender: &Sender<String>,
    base_info: BaseInfo,
    session_id: String,
  ) -> Result<(), JsValue> {
    let rtc = base_info.rtc.clone();
    let answer = rtc.borrow().get_send_answer().await.unwrap();
    let message = CastMessage::Sdp(SdpMessage {
      sdp_type: SdpType::Answer,
      sdp: answer,
    });
    RTCLink::send_signal(sender, base_info, message, session_id).await;
    Ok(())
  }

  pub async fn parse_signal(&self, message: ResponseMessage) {
    let ResponseMessage {
      session_id,
      message,
      ..
    } = message;

    let base_info = self.get_base_info();
    let sender = self.link.sender();
    if let ResponseMessageData::Signal(message) = message {
      let SignalMessage { message, .. } = message;
      match message {
        message::CastMessage::Sdp(message) => {
          let SdpMessage { sdp_type, sdp } = message;
          match sdp_type {
            message::SdpType::Offer => {
              let _ = self.rtc.borrow().receive_offer(sdp).await;
              let _ = RTCLink::send_answer(&sender, base_info, session_id).await;
            }
            message::SdpType::Answer => {
              let _ = self.rtc.borrow().receive_answer(sdp).await;
            }
          }
        }
        message::CastMessage::Ice(message) => {
          let _ = self.rtc.borrow().receive_ice(message);
        }
      }
    }
  }

  pub async fn set_media(
    &self,
    dom: Option<HtmlMediaElement>,
    media_type: &Option<MediaType>,
  ) -> Result<(), JsValue> {
    if let Some(media_type) = media_type {
      let stream = match media_type {
        MediaType::Video => get_user_media(Some("true"), Some("true")),
        MediaType::Audio => get_user_media(Some("true"), None),
      }
      .await
      .ok();
      if let Some(dom) = dom {
        dom.set_src_object(stream.as_ref());
      }
      if let Some(stream) = stream {
        self.rtc.borrow().set_tracks(stream);
      }
    }
    Ok(())
  }

  pub fn send_message(&self, message: ArrayBuffer) {
    let data = self.rtc.borrow().send_message(message);
    log!("send_message result", format!("{:?}", data));
  }

  pub fn create_datachannel(&mut self) {
    self.rtc.borrow_mut().create_datachannel();
  }
}
