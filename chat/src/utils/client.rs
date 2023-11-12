use std::{cell::RefCell, collections::HashMap, rc::Rc};

use gloo_console::log;
use message::{Action, ActionChannel, ActionMessage, ClientAction, Data, GetInfo};

use crate::{
  store::User,
  utils::{RTCLink, Websocket, SDP_SERVER},
};

pub struct Client {
  user: User,
  ws: Rc<RefCell<Websocket>>,
  this: Option<Rc<RefCell<Self>>>,
  channel: ActionChannel<Websocket>,
  links: HashMap<String, RTCLink>,
}

impl Client {
  pub fn new(user: User) -> Rc<RefCell<Self>> {
    let ws = Websocket::new(SDP_SERVER).unwrap();
    let ws_clone = ws.clone();
    let client = Rc::new(RefCell::new(Client {
      user,
      ws,
      links: HashMap::new(),
      this: None,
      channel: ActionChannel::new(ws_clone),
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

    self.channel.set_receive_message(callback);
    self
      .channel
      .send_message(Action::Client(ClientAction::GetInfo(GetInfo)));
  }

  pub fn user(&self) -> User {
    self.user.clone()
  }
}
