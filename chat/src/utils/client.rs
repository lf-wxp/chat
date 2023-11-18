use std::{cell::RefCell, collections::HashMap, rc::Rc};

use gloo_console::log;
use message::{
  Action, ActionChannel, ActionMessage, CallChannel, CallMessage, ClientAction, Data, GetInfo,
  SignalChannel,
};
use wasm_bindgen::JsValue;

use crate::{
  store::User,
  utils::{RTCLink, Websocket, SDP_SERVER},
};

pub struct Client {
  user: User,
  ws: Rc<RefCell<Websocket>>,
  this: Option<Rc<RefCell<Self>>>,
  action_channel: ActionChannel<Websocket>,
  call_channel: CallChannel<Websocket>,
  links: HashMap<String, Rc<RefCell<RTCLink>>>,
}

impl Client {
  pub fn new(user: User) -> Rc<RefCell<Self>> {
    let ws = Websocket::new(SDP_SERVER).unwrap();
    let ws_action = ws.clone();
    let ws_call = ws.clone();
    let client = Rc::new(RefCell::new(Client {
      user,
      ws,
      links: HashMap::new(),
      this: None,
      action_channel: ActionChannel::new(ws_action),
      call_channel: CallChannel::new(ws_call),
    }));
    client.borrow_mut().this = Some(client.clone());
    client.borrow_mut().init();
    client
  }

  fn init(&self) {
    self.bind_ws_event();
    self.response_call();
  }

  fn bind_ws_event(&self) {
    let mut ws_client = self.ws.borrow_mut();
    ws_client.set_onopen(Box::new(move || {
      log!("websocket start");
    }));
  }

  pub fn set_onmessage(&mut self, onmessage: Box<dyn Fn(ActionMessage)>) {
    let client = self.this.clone();
    let callback = Box::new(move |message: ActionMessage| {
      let message_clone = message.clone();
      if let Some(client) = &client {
        if let Some(Data::Client(info)) = message.data {
          client.borrow_mut().user.uuid = info.uuid;
        }
      }
      onmessage(message_clone);
    });

    self.action_channel.set_receive_message(callback);
    self
      .action_channel
      .send_message(Action::Client(ClientAction::GetInfo(GetInfo)));
  }

  pub fn user(&self) -> User {
    self.user.clone()
  }

  fn create_link(&mut self, to: String) -> Result<Rc<RefCell<RTCLink>>, JsValue> {
    let ws = self.ws.clone();
    let signal_channel = Rc::new(RefCell::new(SignalChannel::new(
      self.user.uuid.clone(),
      to.clone(),
      ws,
    )));
    RTCLink::new(signal_channel)
  }

  pub fn response_call(&self) {
    let client = self.this.clone();
    let callback = Box::new(move |message: CallMessage| {
      if let Some(client) = &client {
        let CallMessage { from, .. } = message;
        let link = client.borrow_mut().create_link(from.clone()).unwrap();
        // client.borrow_mut().call_channel.call(self.user.uuid.clone(), from.clone());
        client.borrow_mut().links.insert(from.to_string(), link);
      }
    });
    self.call_channel.set_response_message(callback);
  }

  pub fn call(&mut self, callee: String) -> Result<(), JsValue> {
    let link = self.create_link(callee.clone())?;
    self
      .call_channel
      .call(self.user.uuid.clone(), callee.clone());
    link.borrow().send_offer();
    self.links.insert(callee.to_string(), link);
    Ok(())
  }
}
