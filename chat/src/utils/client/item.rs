use async_broadcast::Sender;
use futures::Future;
use gloo_console::log;
use message::{
  self, Action, ActionMessage, CastMessage, ClientAction, ConnectMessage, GetInfo, ListAction,
  ListMessage, MediaMessage, MediaType, MessageType, RequestMessage, RequestMessageData,
  ResponseMessage, ResponseMessageData, ResponseMessageData::Media, SdpMessage, SdpType,
  SignalMessage, UpdateName,
};
use nanoid::nanoid;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use wasm_bindgen_futures::spawn_local;

use crate::{
  store::User,
  utils::{get_link, query_selector, Link, RTCLink, Request},
};

#[derive(Debug)]
pub struct Client {
  pub user: User,
  links: Rc<RefCell<HashMap<String, RTCLink>>>,
  link: &'static mut Link,
}

impl Client {
  pub fn new(user: User) -> Self {
    let link = get_link().unwrap();

    let client = Client {
      user,
      links: Rc::new(RefCell::new(HashMap::new())),
      link,
    };
    client.watch_message();
    client
  }

  fn watch_message(&self) {
    let mut receiver = self.link.receiver.clone();
    let links = self.links.clone();
    let sender = self.link.sender();
    let uuid = self.user.uuid.clone();
    spawn_local(async move {
      while let Ok(msg) = receiver.recv().await {
        if let Ok(origin_message) = serde_json::from_str::<ResponseMessage>(&msg) {
          let ResponseMessage {
            message,
            message_type,
            session_id,
          } = &origin_message;
          if let ResponseMessageData::Signal(message) = message {
            let SignalMessage { from, .. } = message;
            let links = links.borrow();
            let link = links.get(from).unwrap();
            link.parse_signal(origin_message, &sender).await;
            continue;
          }

          if *message_type == MessageType::Response {
            continue;
          }
          if let ResponseMessageData::Connect(message) = message {
            let ConnectMessage { from, .. } = &message;
            let link = RTCLink::new(from.to_string()).unwrap();
            links.borrow_mut().insert(from.to_string(), link);
            Client::replay_request_connect(&sender, uuid.clone(), from.clone(), session_id.clone())
              .await;
          }
        }
      }
    })
  }

  async fn send(&self, message: RequestMessageData, message_type: MessageType, session_id: String) {
    let sender = self.link.sender();
    Client::send_static(&sender, message, message_type, session_id).await;
  }

  async fn send_static(
    sender: &Sender<String>,
    message: RequestMessageData,
    message_type: MessageType,
    session_id: String,
  ) {
    let message = serde_json::to_string(&RequestMessage {
      message,
      session_id,
      message_type,
    })
    .unwrap();
    let _ = sender.broadcast_direct(message).await;
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

  pub async fn request_media(&mut self, to: String, media_type: MediaType) {
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
    match futures.await {
      Ok(message) => {
        if let Media(MediaMessage {
          media_type,
          from_name,
          from,
          confirm,
          ..
        }) = message
        {
          if confirm.is_some_and(|x| x) {
            self.request_connect(from.clone()).await;
            log!("confirm", from_name, from, format!("{:?}", media_type));
          }
        }
      }
      Err(_) => {
        log!("time out");
      }
    };
  }

  async fn replay_request_media(
    &mut self,
    message: MediaMessage,
    confirm: bool,
    session_id: String,
  ) {
    let MediaMessage {
      media_type, from, ..
    } = message;
    let message = RequestMessageData::Media(MediaMessage {
      from: self.user.uuid.clone(),
      from_name: self.user.name.clone(),
      to: from,
      media_type,
      confirm: Some(confirm),
    });
    self.send(message, MessageType::Response, session_id).await;
  }

  pub async fn reject_request_media(&mut self, message: MediaMessage, session_id: String) {
    self.replay_request_media(message, false, session_id).await;
  }
  pub async fn confirm_request_media(&mut self, message: MediaMessage, session_id: String) {
    self.replay_request_media(message, true, session_id).await;
  }

  pub async fn replay_request_connect(
    sender: &Sender<String>,
    from: String,
    to: String,
    session_id: String,
  ) {
    let message = RequestMessageData::Connect(ConnectMessage { from, to });
    Client::send_static(sender, message, MessageType::Response, session_id).await;
  }

  pub async fn request_connect(&mut self, to: String) {
    let link = RTCLink::new(to.clone()).unwrap();
    let offer = &link.get_send_offer().await.unwrap();
    let session_id = nanoid!();
    let message = RequestMessageData::Connect(ConnectMessage {
      from: self.user.uuid.clone(),
      to: to.clone(),
    });
    let request = Request::new(self.link.sender(), self.link.receiver());
    let futures = request.feature();
    request.request(message);
    match futures.await {
      Ok(_) => {
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
        let _ = self.send(message, MessageType::Request, session_id).await;
      }
      Err(_) => todo!(),
    }
  }
}
