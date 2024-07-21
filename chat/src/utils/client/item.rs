use std::{cell::RefCell, collections::HashMap, rc::Rc, time::Duration};

use async_broadcast::Sender;
use futures::Future;
use gloo_console::log;
use gloo_timers::future::sleep;
use nanoid::nanoid;
use message::{
  self, Action, ActionMessage, CastMessage, ClientAction, ConnectMessage, GetInfo, ListAction, ListMessage, MediaMessage, MediaType, MessageType, RequestMessage, RequestMessageData, ResponseMessage, ResponseMessageData, SdpMessage, SdpType, SignalMessage, UpdateName
};
use wasm_bindgen::JsValue;

use crate::{
  store::User,
  utils::{get_link, query_selector, Link, RTCLink, Request},
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
            from: to.clone(),
            to: from,
            from_name: to,
            media_type,
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

  async fn send(&self, message: RequestMessageData) {
    let session_id = nanoid!();
    let message = serde_json::to_string(&RequestMessage {
      message,
      session_id,
      message_type: MessageType::Request,
    }).unwrap();
    log!("send message 123", &message);
    let _ = self.link.sender().broadcast_direct(message).await;
    log!("after message 123");
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
            from_name: self.user.name.clone(),
            to: from,
            media_type,
            confirm: None,
          });
          self.send(message).await;
        }
      }
      Err(_) => todo!(),
    }
  }

  pub async fn get_init_info(&mut self) -> Option<message::Client> {
    let message = RequestMessageData::Action(Action::Client(ClientAction::GetInfo(GetInfo)));
    let request = Request::new(self.link.sender(), self.link.receiver());
    let futures = request.feature();
    request.request(message);
    if let Ok(ResponseMessageData::Action(ActionMessage::Client(info))) = futures.await {
      self.set_user(info.clone());
      return Some(info);
    }
    None
  }

  pub async fn get_user_list(&mut self) -> Option<ListMessage> {
    let message = RequestMessageData::Action(Action::List(ListAction));
    let request = Request::new(self.link.sender(), self.link.receiver());
    let futures = request.feature();
    request.request(message);
    if let Ok(ResponseMessageData::Action(ActionMessage::ListMessage(list_message))) = futures.await
    {
      return Some(list_message);
    }
    None
  }

  pub fn user(&self) -> User {
    self.user.clone()
  }

  pub fn set_name(&mut self, name: String) {
    self.user.name = name;
  }
  pub fn set_user(&mut self, client: message::Client) {
    self.user = client.into();
  }


  pub fn update_name(
    &mut self,
    name: String,
  ) -> impl Future<Output = Result<ResponseMessageData, ()>> {
    let message =
      RequestMessageData::Action(Action::Client(ClientAction::UpdateName(UpdateName {
        name,
      })));
    let request = Request::new(self.link.sender(), self.link.receiver());
    let futures = request.feature();
    request.request(message);
    futures
  }

  pub fn request_media(
    &mut self,
    to: String,
    media_type: MediaType,
  ) -> impl Future<Output = Result<ResponseMessageData, ()>> {
    log!("user", format!("{:?}", self.user));
    let message = RequestMessageData::Media(MediaMessage {
      from: self.user.uuid.clone(),
      from_name: self.user.name.clone(),
      to: to.clone(),
      media_type,
      confirm: None,
    });
    let request = Request::new(self.link.sender(), self.link.receiver());
    let futures = request.feature();
    request.request(message);
    futures
  }

  async fn replay_request_media(&mut self, message: MediaMessage, confirm: bool) {
    let MediaMessage { media_type, from ,..} = message;
    let message = RequestMessageData::Media(MediaMessage {
      from: self.user.uuid.clone(),
      from_name: self.user.name.clone(),
      to: from,
      media_type,
      confirm: Some(confirm),
    });
    self.send(message).await;
  }

  pub async fn reject_request_media(&mut self, message: MediaMessage) {
    self.replay_request_media(message, false).await;
  }
  pub async fn confirm_request_media(&mut self, message: MediaMessage) {
    self.replay_request_media(message, true).await;
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
