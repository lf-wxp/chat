use std::{cell::RefCell, collections::HashMap, rc::Rc};

use async_broadcast::{broadcast, Receiver, Sender};
use futures::{SinkExt, StreamExt};
use gloo_console::log;
use gloo_net::websocket::{futures::WebSocket, Message};
use message::{
  Action, CastMessage, ClientAction, ConnectMessage,
  GetInfo, MediaMessage, MediaType, RequestMessage, ResponseMessage,
  SdpMessage, SdpType, SignalMessage,
};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

use crate::{
  store::User,
  utils::{Link, SDP_SERVER},
};

pub struct Client {
  pub user: User,
  ws: WebSocket,
  this: Option<Rc<RefCell<Self>>>,
  links: Rc<RefCell<HashMap<String, Link>>>,
  sender: Sender<String>,
  pub receiver: Receiver<String>,
}

impl Client {
  pub fn new(user: User) -> Rc<RefCell<Self>> {
    let ws = WebSocket::open(SDP_SERVER).unwrap();
    let (sender, receiver) = broadcast(10);
    let client = Rc::new(RefCell::new(Client {
      user,
      ws,
      links: Rc::new(RefCell::new(HashMap::new())),
      sender,
      receiver,
      this: None,
    }));
    client.borrow_mut().this = Some(client.clone());
    client
  }

  fn setup(&self) {
    let links = self.links.clone();
    let sender = self.sender.clone();
    let (mut write, mut read) = self.ws.split();
    spawn_local(async move {
      while let Some(msg) = read.next().await {
        match msg {
          Ok(msg) => {
            if let Message::Text(msg) = msg {
              sender.broadcast(msg);
              self.parse_action(msg);
              self.parse_signal(msg);
              self.parse_media(msg);
              self.parse_connect(msg);
            }
          }
          Err(_) => todo!(),
        }
      }
      log!("WebSocket Closed")
    });
    spawn_local(async move {
      while let Some(msg) = self.receiver.next().await {
        write.send(Message::Text(msg));
      }
    });
  }

  fn parse_action(&self, message: String) {}
  fn parse_connect(&self, message: String) {
    let links = self.links.clone();
    match serde_json::from_str::<ResponseMessage>(&message) {
      Ok(msg) => {
        if let ResponseMessage::Connect(message) = msg {
          let ConnectMessage { from, .. } = message;
          let link = Link::new(from).unwrap();
          log!("receive call");
          links.borrow_mut().insert(from.to_string(), link);
        }
      }
      Err(_) => todo!(),
    }
  }

  fn parse_signal(&self, message: String) {
    let links = self.links.clone();
    match serde_json::from_str::<ResponseMessage>(&message) {
      Ok(msg) => if let ResponseMessage::Signal(message) = msg {},
      Err(_) => todo!(),
    }
  }
  fn parse_media(&self, message: String) {
    let links = self.links.clone();
    match serde_json::from_str::<ResponseMessage>(&message) {
      Ok(msg) => if let ResponseMessage::Media(message) = msg {},
      Err(_) => todo!(),
    }
  }

  pub fn get_init_info(&self) {
    let message = RequestMessage::Action(Action::Client(ClientAction::GetInfo(GetInfo)));
    let message = serde_json::to_string(&message).unwrap();
    self.sender.broadcast(message);
  }

  pub fn user(&self) -> User {
    self.user.clone()
  }

  pub fn request_media(&mut self, to: String, media_type: MediaType) {
    let message = RequestMessage::Media(MediaMessage {
      from: self.user.uuid.clone(),
      to: to.clone(),
      media_type,
      expired: None,
      confirm: None,
    });
    let message = serde_json::to_string(&message).unwrap();
    self.sender.broadcast(message);
  }

  pub async fn request_connect(&mut self, to: String) -> Result<(), JsValue> {
    let link = Link::new(to.clone()).unwrap();
    let offer = &link.get_send_offer().await?;

    let message = RequestMessage::Connect(ConnectMessage {
      from: self.user.uuid.clone(),
      to: to.clone(),
    });
    let message = serde_json::to_string(&message).unwrap();
    self.sender.broadcast(message);

    self.links.borrow_mut().insert(to.to_string(), link);

    let message = RequestMessage::Signal(SignalMessage {
      from: self.user.uuid.clone(),
      to: to.clone(),
      message: CastMessage::Sdp(SdpMessage {
        sdp_type: SdpType::Offer,
        sdp: offer.clone(),
      }),
    });
    let message = serde_json::to_string(&message).unwrap();
    self.sender.broadcast(message);
    Ok(())
  }
}
