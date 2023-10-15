use std::{cell::RefCell, rc::Rc};

use gloo_console::log;
use js_sys::JsString;
use wasm_bindgen::{JsCast, JsValue};

use crate::{store::User, utils::{Websocket, WebRTC, SocketMessage}};

pub struct Client {
  user: User, 
  ws: Rc<RefCell<Websocket>>,  
  rtc: Option<WebRTC>,
}

impl Client {
  pub fn new(user: User) -> Self {
    let ws = Websocket::new("ws://127.0.0.1:8888").unwrap();
    Client {
      user,
      ws,
      rtc: None,
    }
  }

  fn bind_ws_event(self) {
    let mut client = self.ws.borrow_mut();
    client.set_onopen(Box::new(move || {
      log!("websocket start");
    }));
    client.set_onmessage(Box::new(|msg: SocketMessage| {
      log!("receive message", format!("{:?}", msg));
    }));
    client.send(SocketMessage::Str(JsString::from("hello"))).unwrap();
  }
}
