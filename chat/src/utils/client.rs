use std::{cell::RefCell, collections::HashMap, rc::Rc};

use futures::{SinkExt, StreamExt};
use gloo_console::log;
use gloo_net::websocket::{futures::WebSocket, Message};
use message::{
  Action, CastMessage, ClientAction, ConnectMessage, GetInfo, MediaMessage, MediaType,
  RequestMessage, ResponseMessage, SdpMessage, SdpType, SignalMessage,
};
use postage::{
  broadcast::{self, Receiver, Sender},
  sink::Sink,
};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

use crate::{
  store::User,
  utils::{Link, SDP_SERVER},
};

use super::query_selector;

pub struct Client {
  pub user: User,
  this: Option<Rc<RefCell<Self>>>,
  links: Rc<RefCell<HashMap<String, Link>>>,
  sender: Sender<String>,
  pub receiver: Receiver<String>,
}

impl Client {
  pub fn new(user: User) -> Rc<RefCell<Self>> {
    let (sender, receiver) = broadcast::channel(10);
    let client = Rc::new(RefCell::new(Client {
      user,
      links: Rc::new(RefCell::new(HashMap::new())),
      sender,
      receiver,
      this: None,
    }));
    client.borrow_mut().this = Some(client.clone());
    client.borrow_mut().setup();
    client
  }

  fn setup(&self) {
    let ws = WebSocket::open(SDP_SERVER).unwrap();
    let (mut write, mut read) = ws.split();
    let client = self.this.clone().unwrap().clone();
    let mut sender = self.sender.clone();
    spawn_local(async move {
      while let Some(msg) = read.next().await {
        match msg {
          Ok(msg) => {
            if let Message::Text(msg) = msg {
              client.borrow_mut().parse_action(&msg);
              client.borrow_mut().parse_signal(&msg);
              client.borrow_mut().parse_media(&msg);
              client.borrow_mut().parse_connect(&msg);
              let _ = sender.blocking_send(msg);
            }
          }
          Err(_) => todo!(),
        }
      }
      log!("WebSocket Closed")
    });
    let mut receiver = self.receiver.clone();
    spawn_local(async move {
      while let Some(msg) = receiver.next().await {
        let _ = write.send(Message::Text(msg));
      }
    });
  }

  fn parse_action(&self, _message: &str) {}
  fn parse_connect(&self, message: &str) {
    let links = self.links.clone();
    match serde_json::from_str::<ResponseMessage>(&message) {
      Ok(msg) => {
        if let ResponseMessage::Connect(message) = msg {
          let ConnectMessage { from, .. } = &message;
          let link = Link::new(from.to_string()).unwrap();
          log!("receive call");
          links.borrow_mut().insert(from.to_string(), link);
        }
      }
      Err(_) => todo!(),
    }
  }

  fn parse_signal(&self, message: &str) {
    let _links = self.links.clone();
    match serde_json::from_str::<ResponseMessage>(&message) {
      Ok(msg) => if let ResponseMessage::Signal(_message) = msg {},
      Err(_) => todo!(),
    }
  }
  fn parse_media(&self, message: &str) {
    let _links = self.links.clone();
    match serde_json::from_str::<ResponseMessage>(&message) {
      Ok(msg) => if let ResponseMessage::Media(_message) = msg {},
      Err(_) => todo!(),
    }
  }

  pub fn get_init_info(&mut self) {
    let message = RequestMessage::Action(Action::Client(ClientAction::GetInfo(GetInfo)));
    let message = serde_json::to_string(&message).unwrap();
    let _ = self.sender.blocking_send(message);
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
    self.sender.blocking_send(message);
  }

  pub async fn request_connect(&mut self, to: String) -> Result<(), JsValue> {
    let link = Link::new(to.clone()).unwrap();
    let offer = &link.get_send_offer().await?;

    let message = RequestMessage::Connect(ConnectMessage {
      from: self.user.uuid.clone(),
      to: to.clone(),
    });
    let message = serde_json::to_string(&message).unwrap();
    let _ = self.sender.send(message).await;
    let dom = query_selector(".local-stream");
    log!("local stream", dom.clone());
    let _ = link.set_local_user_media(dom).await;
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
    let _ = self.sender.send(message).await;
    Ok(())
  }
}
