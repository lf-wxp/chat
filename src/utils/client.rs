use std::{cell::RefCell, rc::Rc};

use gloo_console::log;

use crate::{
  model::{
    Action, CallType, ClientAction, Data, GetInfo, SdpMessage, Transmit, Unicast, WsMessage,
    WsResponse, TransmitMessage,
  },
  store::User,
  utils::{SocketMessage, WebRTC, Websocket, SDP_SERVER},
};

pub struct Client {
  user: User,
  ws: Rc<RefCell<Websocket>>,
  rtc: Option<WebRTC>,
  this: Option<Rc<RefCell<Self>>>,
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
            if let Some(Data::Transmit(message)) = sdp_response.data {
              client.borrow_mut().set_remote_description(message)
            }
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

  fn set_remote_description(&mut self, message: TransmitMessage) {
    if let Some(rtc) = &self.rtc {
      todo!()
    }
  }

  pub fn set_onmessage(&mut self, onmessage: Box<dyn Fn(WsResponse)>) {
    self.onmessage = Some(onmessage);
  }

  pub fn user(&self) -> User {
    self.user.clone()
  }

  pub async fn call(&mut self, to: String, call_type: CallType) {
    let mut ws_client = self.ws.borrow_mut();
    if let Some(rtc) = &mut self.rtc {
      if let Some(sdp) = rtc.sdp().await {
        let action = &WsMessage::Transmit(Transmit::Unicast(Unicast {
          from: self.user.uuid.clone(),
          to,
          message: SdpMessage { call_type, sdp: sdp.to_string() },
        }));
        let message = serde_json::to_string(action).unwrap().into();
        ws_client.send(SocketMessage::Str(message)).unwrap();
      }
    }
  }
}
