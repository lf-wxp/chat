use std::{cell::RefCell, collections::HashMap, rc::Rc, time::Duration};

use futures::{SinkExt, StreamExt};
use gloo_console::log;
use gloo_net::websocket::{futures::WebSocket, Message};
use gloo_timers::future::sleep;
use message::{
  Action, CastMessage, ClientAction, ConnectMessage, GetInfo, MediaMessage, MediaType, MessageType,
  RequestMessage, RequestMessageData, ResponseMessage, ResponseMessageData, SdpMessage, SdpType,
  SignalMessage,
};
use nanoid::nanoid;
use postage::{
  broadcast::{self, Receiver, Sender},
  sink::Sink,
};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

use crate::{
  store::User,
  utils::{query_selector, Link, Request, RequestFuture, SDP_SERVER},
};

async fn parse_media(sender: &mut Sender<String>, message: &str) {
  match serde_json::from_str::<ResponseMessage>(message) {
    Ok(ResponseMessage {
      session_id,
      message,
      message_type,
    }) => {
      if let ResponseMessageData::Media(message) = message {
        let MediaMessage {
          from,
          to,
          media_type,
          ..
        } = message.clone();
        log!(
          "receive media message",
          format!("{}-{}--{:?}", from, to, message_type)
        );
        if let MessageType::Request = message_type {
          let message = ResponseMessageData::Media(MediaMessage {
            from: to,
            to: from,
            media_type,
            expired: None,
            confirm: None,
          });
          let message = serde_json::to_string(&ResponseMessage {
            session_id,
            message,
            message_type: MessageType::Response,
          })
          .unwrap();
          sleep(Duration::from_secs(3)).await;
          let _ = sender.blocking_send(message);
        }
      }
    }
    Err(_) => todo!(),
  }
}
pub struct Client {
  pub user: User,
  links: Rc<RefCell<HashMap<String, Link>>>,
  read_sender: Sender<String>,
  read_receiver: Receiver<String>,
  write_sender: Sender<String>,
  write_receiver: Receiver<String>,
}

impl Client {
  pub fn new(user: User) -> Self {
    let (read_sender, read_receiver) = broadcast::channel(10);
    let (write_sender, write_receiver) = broadcast::channel(10);
    let client = Client {
      user,
      links: Rc::new(RefCell::new(HashMap::new())),
      read_sender,
      read_receiver,
      write_sender,
      write_receiver,
    };
    client.setup();
    client
  }

  fn setup(&self) {
    let ws = WebSocket::open(SDP_SERVER).unwrap();
    let (mut write, mut read) = ws.split();
    let mut sender = self.read_sender.clone();
    let mut write_sender = self.write_sender.clone();
    spawn_local(async move {
      while let Some(msg) = read.next().await {
        match msg {
          Ok(msg) => {
            if let Message::Text(msg) = msg {
              log!("broadcast msg receive", &msg);
              let _ = sender.blocking_send(msg.clone());
              parse_media(&mut write_sender, &msg).await;
            }
          }
          Err(_) => todo!(),
        }
      }
      log!("WebSocket Closed")
    });
    let mut receiver = self.write_receiver.clone();
    spawn_local(async move {
      while let Some(msg) = receiver.next().await {
        log!("broadcast msg", &msg);
        let _ = write.send(Message::Text(msg)).await;
      }
    });
  }

  fn parse_action(&self, _message: &str) {}
  fn parse_connect(&self, message: &str) {
    let links = self.links.clone();
    match serde_json::from_str::<ResponseMessage>(&message) {
      Ok(ResponseMessage {
        message,
        session_id,
        ..
      }) => {
        if let ResponseMessageData::Connect(message) = message {
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
      Ok(ResponseMessage {
        session_id,
        message,
        ..
      }) => if let ResponseMessageData::Signal(_message) = message {},
      Err(_) => todo!(),
    }
  }
  fn parse_media(&mut self, message: &str) {
    let _links = self.links.clone();
    match serde_json::from_str::<ResponseMessage>(message) {
      Ok(ResponseMessage {
        session_id,
        message,
        ..
      }) => {
        if let ResponseMessageData::Media(message) = message {
          let MediaMessage {
            from,
            to,
            media_type,
            ..
          } = message.clone();
          log!("receive media message", format!("{:?}", message.clone()));
          let message = RequestMessageData::Media(MediaMessage {
            from: self.user.uuid.clone(),
            to: from,
            media_type,
            expired: None,
            confirm: None,
          });
          let message = serde_json::to_string(&message).unwrap();
          let _ = self.write_sender.blocking_send(message);
        }
      }
      Err(_) => todo!(),
    }
  }

  pub fn get_init_info(&mut self) {
    let message = RequestMessageData::Action(Action::Client(ClientAction::GetInfo(GetInfo)));
    let message = serde_json::to_string(&RequestMessage {
      session_id: nanoid!(),
      message,
      message_type: MessageType::Request,
    })
    .unwrap();
    let _ = self.write_sender.blocking_send(message);
  }

  pub fn user(&self) -> User {
    self.user.clone()
  }

  pub fn receiver(&self) -> Receiver<String> {
    self.read_receiver.clone()
  }

  pub fn request_media(&mut self, to: String, media_type: MediaType) -> RequestFuture {
    let message = RequestMessageData::Media(MediaMessage {
      from: self.user.uuid.clone(),
      to: to.clone(),
      media_type,
      expired: None,
      confirm: None,
    });
    let sender = self.write_sender.clone();
    let receiver = self.read_receiver.clone();
    let mut request = Request::new(sender, receiver);
    request.request(message)
  }

  pub async fn request_connect(&mut self, to: String) -> Result<(), JsValue> {
    let link = Link::new(to.clone()).unwrap();
    let offer = &link.get_send_offer().await?;

    let message = RequestMessageData::Connect(ConnectMessage {
      from: self.user.uuid.clone(),
      to: to.clone(),
    });
    let message = serde_json::to_string(&message).unwrap();
    let _ = self.write_sender.send(message).await;
    let dom = query_selector(".local-stream");
    log!("local stream", dom.clone());
    let _ = link.set_local_user_media(dom).await;
    self.links.borrow_mut().insert(to.to_string(), link);

    let message = RequestMessageData::Signal(SignalMessage {
      from: self.user.uuid.clone(),
      to: to.clone(),
      message: CastMessage::Sdp(SdpMessage {
        sdp_type: SdpType::Offer,
        sdp: offer.clone(),
      }),
    });
    let message = serde_json::to_string(&message).unwrap();
    let _ = self.write_sender.send(message).await;
    Ok(())
  }
}
