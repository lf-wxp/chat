use async_broadcast::{Receiver, Sender};
use gloo_console::log;
use js_sys::JSON;
use message::{
  CastMessage, ConnectMessage, ConnectState, MessageType, RequestMessage, RequestMessageData,
  ResponseMessage, ResponseMessageData, SdpMessage, SdpType, SignalMessage,
};
use nanoid::nanoid;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlMediaElement, MediaStream, RtcIceConnectionState, RtcPeerConnection};
use yew::Event;

use crate::utils::{get_target, get_user_media, query_selector};

use super::{
  rtc::{ChannelMessage, WebRTC},
  ConnectFuture,
};

#[derive(Debug)]
pub struct RTCLink {
  id: String,
  remote_id: String,
  remote_media: Rc<RefCell<MediaStream>>,
  sender: Sender<String>,
  receiver: Receiver<String>,
  ready: Rc<RefCell<bool>>,
  rtc: WebRTC,
}

impl RTCLink {
  pub fn new(
    id: String,
    remote_id: String,
    sender: Sender<String>,
    receiver: Receiver<String>,
  ) -> Result<Self, JsValue> {
    let rtc = WebRTC::new()?;
    let remote_media = MediaStream::new()?;
    let link = RTCLink {
      id,
      remote_id,
      rtc,
      sender,
      receiver,
      ready: Rc::new(RefCell::new(false)),
      remote_media: Rc::new(RefCell::new(remote_media)),
    };
    link.watch_rtc_event();
    Ok(link)
  }

  fn watch_rtc_event(&self) {
    let mut receiver = self.rtc.message_receiver.clone();
    let sender = self.sender.clone();
    let from = self.id.clone();
    let to = self.remote_id.clone();
    let remote_media = self.remote_media.clone();
    let ready = self.ready.clone();
    log!("watch ", from.clone(), to.clone());
    spawn_local(async move {
      while let Ok(msg) = receiver.recv().await {
        match msg {
          ChannelMessage::ErrorEvent => {}
          ChannelMessage::TrackEvent(ev) => {
            remote_media.borrow_mut().add_track(&ev.track());
            if let Some(dom) = query_selector::<HtmlMediaElement>(".remote-stream") {
              dom.set_src_object(Some(&remote_media.borrow()));
            }
          }
          ChannelMessage::DataChannelEvent(ev) => {}
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
          ChannelMessage::DataChannelMessage(ev) => {}
          ChannelMessage::IceConnectionStateChange(ev) => {
            let target = get_target::<Event, RtcPeerConnection>(ev);
            if target.is_some() {
              let is_connected =
                target.unwrap().ice_connection_state() == RtcIceConnectionState::Connected;
              *ready.borrow_mut() = is_connected;
              if is_connected {
                RTCLink::send_connect_static(
                  &sender,
                  from.clone(),
                  to.clone(),
                  ConnectState::CONNECTED,
                  nanoid!(),
                )
                .await;
              }
            }
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
    session_id: String,
  ) {
    let message = serde_json::to_string(&RequestMessage {
      message: RequestMessageData::Connect(ConnectMessage { from, to, state }),
      session_id,
      message_type: MessageType::Request,
    })
    .unwrap();
    let _ = sender.broadcast_direct(message).await;
  }

  async fn send_signal(&self, message: CastMessage, session_id: String) {
    let sender = self.sender.clone();
    let from = self.id.clone();
    let to = self.remote_id.clone();
    RTCLink::send_signal_static(&sender, from, to, message, session_id).await;
  }

  pub async fn connect(&self) {
    let receiver = self.receiver.clone();
    let future = ConnectFuture::new(self.remote_id.clone(), receiver);
    let _ = self.send_offer().await;
    let a = future.await;
    log!("connected");
  }

  pub async fn send_offer(&self) -> Result<(), JsValue> {
    let offer = self.rtc.get_send_offer().await?;
    let message = CastMessage::Sdp(SdpMessage {
      sdp_type: SdpType::Offer,
      sdp: offer,
    });
    self.send_signal(message, nanoid!()).await;
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
              let answer = self.rtc.get_send_answer().await.unwrap();
              let message = CastMessage::Sdp(SdpMessage {
                sdp_type: SdpType::Answer,
                sdp: answer,
              });
              self.send_signal(message, session_id).await;
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

  pub async fn set_local_user_media(&self, dom: Option<HtmlMediaElement>) -> Result<(), JsValue> {
    let stream = get_user_media(
      // Some("{ device_id: 'default',echo_cancellation: true }"),
      None,
      Some("true"),
    )
    .await
    .ok();
    if let Some(dom) = dom {
      dom.set_src_object(stream.as_ref());
    }
    if let Some(stream) = stream {
      self.rtc.set_tracks(stream);
    }
    Ok(())
  }
}
