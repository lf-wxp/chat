use async_broadcast::Sender;
use gloo_console::log;
use js_sys::JSON;
use message::{
  CastMessage, ConnectMessage, ConnectState, MediaType, MessageType, RequestMessage,
  RequestMessageData, ResponseMessage, ResponseMessageData, SdpMessage, SdpType, SignalMessage,
};
use nanoid::nanoid;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use wasm_bindgen_futures::spawn_local;
use web_sys::{
  HtmlMediaElement, MediaStream, MessageEvent, RtcIceConnectionState, RtcPeerConnection,
};
use yew::Event;

use crate::utils::{get_link, get_target, get_user_media, query_selector, to_connect_state, Link};

use super::{
  rtc::{ChannelMessage, WebRTC},
  Connect, ConnectError,
};

#[derive(Debug)]
pub enum RtcType {
  Caller,
  Callee,
}
#[derive(Debug)]
pub struct RTCLink {
  id: String,
  remote_id: String,
  remote_media: Rc<RefCell<MediaStream>>,
  ready: Rc<RefCell<bool>>,
  link: &'static mut Link,
  rtc: WebRTC,
  rtc_type: RtcType,
}

impl RTCLink {
  pub fn new(id: String, remote_id: String, rtc_type: RtcType) -> Result<Self, JsValue> {
    let rtc = WebRTC::new()?;
    let link = get_link().unwrap();
    let remote_media = MediaStream::new()?;
    let link = RTCLink {
      id,
      remote_id,
      rtc,
      ready: Rc::new(RefCell::new(false)),
      remote_media: Rc::new(RefCell::new(remote_media)),
      link,
      rtc_type,
    };
    link.watch_rtc_event();
    Ok(link)
  }

  fn watch_rtc_event(&self) {
    let mut receiver = self.rtc.message_receiver.clone();
    let sender = self.link.sender();
    let from = self.id.clone();
    let to = self.remote_id.clone();
    let remote_media = self.remote_media.clone();
    let ready = self.ready.clone();
    spawn_local(async move {
      while let Ok(msg) = receiver.recv().await {
        match msg {
          ChannelMessage::ErrorEvent => {}
          ChannelMessage::TrackEvent(ev) => {
            log!("track event");
            remote_media.borrow_mut().add_track(&ev.track());
            if let Some(dom) = query_selector::<HtmlMediaElement>(".remote-stream") {
              dom.set_src_object(Some(&remote_media.borrow()));
            }
          }
          ChannelMessage::DataChannelEvent(ev) => {
            log!("data channel data");
            let channel = ev.channel();
            let message_callback = Closure::wrap(Box::new(move |ev: MessageEvent| {
              log!("receiver message", ev);
            }) as Box<dyn FnMut(_)>);
            let _ = channel.add_event_listener_with_callback(
              "message",
              message_callback.as_ref().unchecked_ref(),
            );
            message_callback.forget(); // 防止闭包在事件监听器结束时被销毁
          }
          ChannelMessage::IceEvent(ev) => {
            let ice = ev.candidate().map(|candidate| {
              JSON::stringify(&candidate.to_json())
                .unwrap()
                .as_string()
                .unwrap()
            });
            if ice.is_some() {
              let message = CastMessage::Ice(ice.unwrap());
              RTCLink::send_signal_static(&sender, from.clone(), to.clone(), message, nanoid!())
                .await;
            }
          }
          ChannelMessage::DataChannelCloseEvent => {}
          ChannelMessage::DataChannelErrorEvent => {}
          ChannelMessage::DataChannelMessage(ev) => {
            log!("receive channel message", ev);
          }
          ChannelMessage::IceConnectionStateChange(ev) => {
            let target = get_target::<Event, RtcPeerConnection>(ev);
            let state = target.as_ref().unwrap().ice_connection_state();
            log!("ice change", format!("{:?}", &state));
            if target.is_some() {
              let is_connected = state == RtcIceConnectionState::Connected;
              *ready.borrow_mut() = is_connected;
              RTCLink::send_connect_static(
                &sender,
                from.clone(),
                to.clone(),
                to_connect_state(state),
                None,
                nanoid!(),
              )
              .await;
            }
          }
          ChannelMessage::Negotiationneeded(ev) => {
            log!("track negotiation");
          }
          ChannelMessage::DataChannelOpenEvent(ev) => {
            log!("data change open");
          }
        }
      }
    })
  }

  async fn send_signal_static(
    sender: &Sender<String>,
    from: String,
    to: String,
    message: CastMessage,
    session_id: String,
  ) {
    let message = serde_json::to_string(&RequestMessage {
      message: RequestMessageData::Signal(SignalMessage { from, to, message }),
      session_id,
      message_type: MessageType::Request,
    })
    .unwrap();
    let _ = sender.broadcast_direct(message).await;
  }

  async fn send_connect_static(
    sender: &Sender<String>,
    from: String,
    to: String,
    state: ConnectState,
    media_type: Option<MediaType>,
    session_id: String,
  ) {
    let message = serde_json::to_string(&RequestMessage {
      message: RequestMessageData::Connect(ConnectMessage {
        from,
        to,
        state,
        media_type,
      }),
      session_id,
      message_type: MessageType::Request,
    })
    .unwrap();
    let _ = sender.broadcast_direct(message).await;
  }

  async fn send_signal(&self, message: CastMessage, session_id: String) {
    let sender = self.link.sender();
    let from = self.id.clone();
    let to = self.remote_id.clone();
    RTCLink::send_signal_static(&sender, from, to, message, session_id).await;
  }

  pub fn is_ready(&self) -> bool {
    *self.ready.clone().borrow()
  }

  pub async fn connect(&self) -> Result<(), ConnectError> {
    let receiver = self.link.receiver();
    let send_future = self.send_offer();
    let connect = Connect::new(self.remote_id.clone(), receiver);
    connect.connect(send_future).await
  }

  pub async fn send_offer(&self) -> Result<(), JsValue> {
    let offer = self.rtc.get_send_offer().await?;
    let message = CastMessage::Sdp(SdpMessage {
      sdp_type: SdpType::Offer,
      sdp: offer,
    });
    self.send_signal(message.clone(), nanoid!()).await;
    Ok(())
  }

  pub async fn send_answer(&self, session_id: String) -> Result<(), JsValue> {
    let answer = self.rtc.get_send_answer().await.unwrap();
    let message = CastMessage::Sdp(SdpMessage {
      sdp_type: SdpType::Answer,
      sdp: answer,
    });
    self.send_signal(message.clone(), session_id).await;
    Ok(())
  }

  pub async fn parse_signal(&self, message: ResponseMessage) {
    let ResponseMessage {
      session_id,
      message,
      ..
    } = message;

    if let ResponseMessageData::Signal(message) = message {
      let SignalMessage { message, .. } = message;
      match message {
        message::CastMessage::Sdp(message) => {
          let SdpMessage { sdp_type, sdp } = message;
          match sdp_type {
            message::SdpType::Offer => {
              let _ = self.rtc.receive_offer(sdp).await;
              let _ = self.send_answer(session_id).await;
            }
            message::SdpType::Answer => {
              let _ = self.rtc.receive_answer(sdp).await;
            }
          }
        }
        message::CastMessage::Ice(message) => {
          let _ = self.rtc.receive_ice(message);
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
        self.rtc.set_tracks(stream);
      }
    }
    Ok(())
  }

  pub fn send_message(&self, message: String) {
    let data = self.rtc.send_message(message);
    log!("send_message result", format!("{:?}", data));
  }
}
