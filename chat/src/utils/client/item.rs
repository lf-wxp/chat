use std::{cell::RefCell, collections::HashMap, rc::Rc, time::Duration};

use async_broadcast::Sender;
use gloo_console::log;
use gloo_timers::future::sleep;
use message::{
  Action, CastMessage, ClientAction, ConnectMessage, GetInfo, MediaMessage, MediaType, MessageType,
  RequestMessage, RequestMessageData, ResponseMessage, ResponseMessageData, SdpMessage, SdpType,
  SignalMessage, UpdateName,
};
use nanoid::nanoid;
use wasm_bindgen::JsValue;

use crate::{
  store::User,
  utils::{future::RequestFuture, get_link, query_selector, Link, RTCLink, Request},
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
          let _ = sender.broadcast_direct(message).await;
        }
      }
    }
    Err(_) => todo!(),
  }
}

#[derive(Debug)]
pub struct Client {
  pub user: User,
  links: Rc<RefCell<HashMap<String, RTCLink>>>,
  link: &'static mut Link,
}

impl Client {
  pub fn new(user: User) -> Self {
    let link = get_link().unwrap();

    Client {
      user,
      links: Rc::new(RefCell::new(HashMap::new())),
      link,
    }
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
          let link = RTCLink::new(from.to_string()).unwrap();
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
  async fn parse_media(&mut self, message: &str) {
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
          let _ = self.link.sender().broadcast_direct(message).await;
        }
      }
      Err(_) => todo!(),
    }
  }

  pub async fn get_init_info(&mut self) {
    let message = RequestMessageData::Action(Action::Client(ClientAction::GetInfo(GetInfo)));
    let message = serde_json::to_string(&RequestMessage {
      session_id: nanoid!(),
      message,
      message_type: MessageType::Request,
    })
    .unwrap();
    let _ = self.link.sender().broadcast_direct(message).await;
  }

  pub fn user(&self) -> User {
    self.user.clone()
  }

  pub fn update_name(&mut self, name: String) -> RequestFuture {
    let message =
      RequestMessageData::Action(Action::Client(ClientAction::UpdateName(UpdateName {
        name,
      })));
    let mut request = Request::new(self.link.sender(), self.link.receiver());
    let futures = request.feature();
    request.request(message);
    futures
  }

  pub fn request_media(&mut self, to: String, media_type: MediaType) -> RequestFuture {
    let message = RequestMessageData::Media(MediaMessage {
      from: self.user.uuid.clone(),
      to: to.clone(),
      media_type,
      expired: None,
      confirm: None,
    });
    let mut request = Request::new(self.link.sender(), self.link.receiver());
    let futures = request.feature();
    request.request(message);
    futures
  }

  pub async fn request_connect(&mut self, to: String) -> Result<(), JsValue> {
    let link = RTCLink::new(to.clone()).unwrap();
    let offer = &link.get_send_offer().await?;

    let message = RequestMessageData::Connect(ConnectMessage {
      from: self.user.uuid.clone(),
      to: to.clone(),
    });
    let message = serde_json::to_string(&message).unwrap();
    let _ = self.link.sender().broadcast(message).await;
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
    let _ = self.link.sender().broadcast(message).await;
    Ok(())
  }
}
