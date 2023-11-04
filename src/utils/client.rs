use std::{cell::RefCell, rc::Rc};

use gloo_console::log;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlMediaElement;

use crate::{
  model::{
    Action, CallType, ClientAction, Data, GetInfo, SdpMessage, Transmit, TransmitMessage, Unicast,
    WsMessage, WsResponse,
  },
  store::User,
  utils::{SocketMessage, WebRTC, Websocket, SDP_SERVER},
};

use super::set_dom_stream;

pub struct Client {
  user: User,
  ws: Rc<RefCell<Websocket>>,
  rtc: Option<WebRTC>,
  this: Option<Rc<RefCell<Self>>>,
  to: Option<String>,
  onmessage: Option<Box<dyn Fn(WsResponse)>>,
}

impl Client {
  pub fn new(user: User) -> Rc<RefCell<Self>> {
    let ws = Websocket::new(SDP_SERVER).unwrap();
    let rtc = WebRTC::new().ok();
    let client = Rc::new(RefCell::new(Client {
      user,
      ws,
      rtc,
      this: None,
      to: None,
      onmessage: None,
    }));
    client.borrow_mut().this = Some(client.clone());
    client.borrow_mut().bind_ws_event();
    client
  }

  fn bind_ws_event(&self) {
    let mut ws_client = self.ws.borrow_mut();
    ws_client.set_onopen(Box::new(move || {
      log!("websocket start");
    }));
    let client = self.this.clone();
    ws_client.set_onmessage(Box::new(move |msg: SocketMessage| {
      log!("receive message", format!("{:?}", msg));
      if let SocketMessage::Str(msg) = msg {
        if let Ok(sdp_response) =
          serde_json::from_str::<WsResponse>(&msg.as_string().expect("error"))
        {
          if let Some(client) = &client {
            if let Some(onmessage) = &client.borrow().onmessage {
              log!("onmessage", format!("{:?}", msg));
              onmessage(sdp_response.clone());
            }
            if let Some(Data::ClientInfo(info)) = sdp_response.data.clone() {
              client.borrow_mut().update_user_uuid(info.uuid);
            }
          }
        }
        if let Ok(transmit_message) =
          serde_json::from_str::<TransmitMessage>(&msg.as_string().expect("error"))
        {
          if let Some(client) = &client {
            let SdpMessage { call_type, sdp } = transmit_message.message;
            let client_clone = client.clone();
            spawn_local(async move {
              match call_type {
                CallType::Answer => {
                  client_clone.borrow_mut().set_remote_description(sdp).await;
                }
                CallType::Offer => {
                  client_clone.borrow_mut().reciprocate(sdp.clone()).await;
                }
              }
            });
          }
        }
      }
    }));
    let action = &WsMessage::Action(Action::Client(ClientAction::GetInfo(GetInfo)));
    let message = serde_json::to_string(action).unwrap().into();
    ws_client.send(SocketMessage::Str(message)).unwrap();
  }

  fn update_user_uuid(&mut self, uuid: String) {
    self.user.uuid = uuid;
  }

  async fn set_remote_description(&mut self, sdp: String) {
    if let Some(rtc) = &mut self.rtc {
      let _ = rtc.receive_answer(sdp).await;
    }
  }

  pub fn set_onmessage(&mut self, onmessage: Box<dyn Fn(WsResponse)>) {
    self.onmessage = Some(onmessage);
  }

  pub fn set_remote_stream(&mut self, dom: HtmlMediaElement) {
    if let Some(rtc) = &self.rtc {
      rtc.set_remote_stream_dom(dom);
    }
  }

  pub fn user(&self) -> User {
    self.user.clone()
  }

  pub async fn reciprocate(&mut self, sdp: String) {
    let to = self.to.clone().unwrap_or("".to_owned());
    let mut ws_client = self.ws.borrow_mut();
    if let Some(rtc) = &mut self.rtc {
      if let Ok(sdp) = rtc.receive_and_emit_offer(sdp).await {
        let action = &WsMessage::Transmit(Transmit::Unicast(Unicast {
          from: self.user.uuid.clone(),
          to,
          message: SdpMessage {
            call_type: CallType::Answer,
            sdp: sdp.to_string(),
          },
        }));
        let message = serde_json::to_string(action).unwrap().into();
        ws_client.send(SocketMessage::Str(message)).unwrap();
      }
    }
  }

  pub async fn call(&mut self, to: String) {
    self.to = Some(to.clone());
    let mut ws_client = self.ws.borrow_mut();
    if let Some(rtc) = &mut self.rtc {
      let _ = rtc.set_stream().await;
      let _ = rtc.attach_stream();
      if let Some(sdp) = rtc.sdp().await {
        let action = &WsMessage::Transmit(Transmit::Unicast(Unicast {
          from: self.user.uuid.clone(),
          to,
          message: SdpMessage {
            call_type: CallType::Offer,
            sdp: sdp.to_string(),
          },
        }));
        let message = serde_json::to_string(action).unwrap().into();
        ws_client.send(SocketMessage::Str(message)).unwrap();
        set_dom_stream(".local-stream", rtc.stream.as_ref());
      }
    }
  }
}
