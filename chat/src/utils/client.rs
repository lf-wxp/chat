use std::{cell::RefCell, collections::HashMap, rc::Rc};

use gloo_console::log;
use message::{
  Action, ActionChannel, ActionMessage, ClientAction, ConnectChannel, ConnectMessage, Data,
  GetInfo, MediaChannel, MediaMessage, MediaType, SignalChannel,
};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlMediaElement;

use crate::{
  store::User,
  utils::{query_selector, RTCLink, Websocket, SDP_SERVER},
};

pub struct Client {
  user: User,
  ws: Rc<RefCell<Websocket>>,
  this: Option<Rc<RefCell<Self>>>,
  action_channel: ActionChannel<Websocket>,
  connect_channel: ConnectChannel<Websocket>,
  media_channel: MediaChannel<Websocket>,
  links: HashMap<String, Rc<RefCell<RTCLink>>>,
}

impl Client {
  pub fn new(user: User) -> Rc<RefCell<Self>> {
    let ws = Websocket::new(SDP_SERVER);
    let ws_action = ws.clone();
    let ws_connect = ws.clone();
    let ws_media = ws.clone();
    let client = Rc::new(RefCell::new(Client {
      user,
      ws,
      links: HashMap::new(),
      this: None,
      action_channel: ActionChannel::new(ws_action),
      connect_channel: ConnectChannel::new(ws_connect),
      media_channel: MediaChannel::new(ws_media),
    }));
    client.borrow_mut().this = Some(client.clone());
    client.borrow_mut().bind_event();
    client
  }

  fn bind_event(&self) {
    self.on_connect();
    self.on_media();
  }

  pub fn set_onmessage(&mut self, onmessage: Box<dyn Fn(ActionMessage)>) {
    let client = self.this.clone();
    let callback = Box::new(move |message: ActionMessage| {
      let message_clone = message.clone();
      log!("action list");
      if let Some(client) = &client {
        if let Some(Data::Client(info)) = message.data {
          client.borrow_mut().user.uuid = info.uuid;
        }
      }
      onmessage(message_clone);
    });

    self.action_channel.set_response_message(callback);
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

  pub fn on_connect(&self) {
    let client = self.this.clone();
    let callback = Box::new(move |message: ConnectMessage| {
      if let Some(client) = &client {
        let ConnectMessage { from, .. } = message;
        let link = client.borrow_mut().create_link(from.clone()).unwrap();
        log!("receive call");
        // let link_clone = link.clone();
        // spawn_local(async move {
        //   let dom = query_selector::<HtmlMediaElement>(".local-stream");
        //   link_clone.borrow().set_local_user_media(dom).await.unwrap();
        // });
        client.borrow_mut().links.insert(from.to_string(), link);
      }
    });
    self.connect_channel.set_response_message(callback);
  }

  pub fn request_media(&mut self, to: String, media_type: MediaType) {
    self.media_channel.send_message(MediaMessage {
      from: self.user.uuid.clone(),
      to: to.clone(),
      media_type,
      expired: None,
      confirm: None,
    });
  }

  fn on_media(&self) {
    let client = self.this.clone().unwrap();
    let callback = Box::new(move |message: MediaMessage| {
      let MediaMessage {
        confirm, from, to, ..
      } = message.clone();
      client.borrow().set_local_stream(from);
    });
    self.media_channel.set_response_message(callback);
  }

  fn set_local_stream(&self, key: String) {
    let client = self.this.clone().unwrap();
    let link = client.borrow_mut().links.get_mut(&key).unwrap().clone();
    spawn_local(async move {
      let dom = query_selector::<HtmlMediaElement>(".local-stream");
      link.borrow().set_local_user_media(dom).await.unwrap();
    });
  }

  pub async fn connect(&mut self, to: String) -> Result<(), JsValue> {
    let link = self.create_link(to.clone())?;
    self.connect_channel.send_message(ConnectMessage {
      from: self.user.uuid.clone(),
      to: to.clone(),
    });
    self.links.insert(to.to_string(), link.clone());
    let link_clone = link.clone();
    let dom = query_selector::<HtmlMediaElement>(".local-stream");
    // link_clone.borrow().set_local_user_media(dom).await.unwrap();
    link_clone.borrow().send_offer().await.unwrap();
    Ok(())
  }
}
