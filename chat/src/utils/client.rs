use std::{cell::RefCell, rc::Rc};

use gloo_console::log;
use message::{
  Action, CastMessage, ClientAction, Data, GetInfo, RequestMessage, ResponseMessage, SdpMessage,
  SdpType, Transmit, Unicast,
};
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlMediaElement;

use crate::{
  store::User,
  utils::{SocketMessage, WebRTC, Websocket, SDP_SERVER},
};

use super::set_dom_stream;

pub struct Client {
  user: User,
  ws: Rc<RefCell<Websocket>>,
  rtc: Option<WebRTC>,
  this: Option<Rc<RefCell<Self>>>,
  to: Rc<RefCell<String>>,
  onmessage: Option<Box<dyn Fn(ResponseMessage)>>,
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
      to: Rc::new(RefCell::new("".to_string())),
      onmessage: None,
    }));
    client.borrow_mut().set_onice();
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
      if let SocketMessage::Str(msg) = msg {
        log!(
          "receive message",
          format!(
            "{:?} {:?}",
            msg,
            serde_json::from_str::<ResponseMessage>(&msg.as_string().expect("error"))
          )
        );
        if let Ok(response) =
          serde_json::from_str::<ResponseMessage>(&msg.as_string().expect("error"))
        {
          log!("onmessage", format!("{:?}", msg));
          if let Some(client) = &client {
            if let Some(onmessage) = &client.borrow().onmessage {
              onmessage(response.clone());
            }
          }
          match response {
            ResponseMessage::Action(action) => {
              if let Some(client) = &client {
                if let Some(Data::Client(info)) = action.data {
                  client.borrow_mut().update_user_uuid(info.uuid);
                }
              }
            }
            ResponseMessage::Transmit(transmit) => {
              if let Some(client) = &client {
                if let CastMessage::Sdp(SdpMessage { sdp_type, sdp }) = transmit.message.clone() {
                  let client_clone = client.clone();
                  spawn_local(async move {
                    match sdp_type {
                      SdpType::Answer => {
                        client_clone
                          .borrow_mut()
                          .set_remote_description(sdp, transmit.from)
                          .await;
                      }
                      SdpType::Offer => {
                        client_clone
                          .borrow_mut()
                          .reciprocate(sdp.clone(), transmit.from)
                          .await;
                      }
                    }
                  });
                }
                if let CastMessage::Ice(ice) = transmit.message {
                  let client_clone = client.clone();
                  client_clone.borrow_mut().add_ice_candidate(ice);
                }
              }
            }
          }
        }
      }
    }));
    let action = &RequestMessage::Action(Action::Client(ClientAction::GetInfo(GetInfo)));
    let message = serde_json::to_string(action).unwrap().into();
    ws_client.send(SocketMessage::Str(message)).unwrap();
  }

  fn set_onice(&mut self) {
    let ws_client = self.ws.clone();
    let from = self.user.uuid.clone();
    let to = self.to.clone();
    if let Some(rtc) = &self.rtc {
      rtc.set_onice(Box::new(move |ice: String| {
        log!("set-ice", to.borrow().clone());

        let ws_client = ws_client.clone();
        let from = from.clone();
        let to = to.clone();
        let on_timeout = move || {
          let action = &RequestMessage::Transmit(Transmit::Unicast(Unicast {
            from: from.clone(),
            to: to.borrow().clone(),
            message: CastMessage::Ice(ice),
          }));
          let message = serde_json::to_string(action).unwrap().into();
          let ws_client = ws_client.clone();
          ws_client
            .borrow_mut()
            .send(SocketMessage::Str(message))
            .unwrap();
        };
        let time = gloo_timers::callback::Timeout::new(1000, on_timeout);
        time.forget();
      }));
    }
  }

  fn update_user_uuid(&mut self, uuid: String) {
    self.user.uuid = uuid;
  }

  async fn set_remote_description(&mut self, sdp: String, from: String) {
    *self.to.borrow_mut() = from;
    if let Some(rtc) = &mut self.rtc {
      let _ = rtc.receive_answer(sdp).await;
    }
  }

  fn add_ice_candidate(&mut self, ice_candidate: String) {
    if let Some(rtc) = &mut self.rtc {
      rtc.add_ice_candidate(ice_candidate);
    }
  }

  pub fn set_onmessage(&mut self, onmessage: Box<dyn Fn(ResponseMessage)>) {
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

  pub async fn reciprocate(&mut self, sdp: String, from: String) {
    let to = from.clone();
    *self.to.borrow_mut() = from;
    let mut ws_client = self.ws.borrow_mut();
    if let Some(rtc) = &mut self.rtc {
      let _ = rtc.set_stream().await;
      // let _ = rtc.attach_stream();
      if let Ok(sdp) = rtc.receive_and_emit_offer(sdp).await {
        log!("reciprocate");
        let action = &RequestMessage::Transmit(Transmit::Unicast(Unicast {
          from: self.user.uuid.clone(),
          to,
          message: CastMessage::Sdp(SdpMessage {
            sdp_type: SdpType::Answer,
            sdp: sdp.to_string(),
          }),
        }));
        let message = serde_json::to_string(action).unwrap().into();
        ws_client.send(SocketMessage::Str(message)).unwrap();
        let _ = rtc.attach_stream();
        set_dom_stream(".local-stream", rtc.stream.as_ref());
      }
    }
  }

  pub async fn call(&mut self, to: String) {
    *self.to.borrow_mut() = to.clone();
    let mut ws_client = self.ws.borrow_mut();
    if let Some(rtc) = &mut self.rtc {
      let _ = rtc.set_stream().await;
      let _ = rtc.attach_stream();
      if let Some(sdp) = rtc.sdp().await {
        let action = &RequestMessage::Transmit(Transmit::Unicast(Unicast {
          from: self.user.uuid.clone(),
          to,
          message: CastMessage::Sdp(SdpMessage {
            sdp_type: SdpType::Offer,
            sdp: sdp.to_string(),
          }),
        }));
        let message = serde_json::to_string(action).unwrap().into();
        ws_client.send(SocketMessage::Str(message)).unwrap();
        set_dom_stream(".local-stream", rtc.stream.as_ref());
      }
    }
  }
}
