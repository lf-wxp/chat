use async_broadcast::Sender;
use futures::Future;
use gloo_console::log;
use message::{
  self, Action, ActionMessage, ClientAction, ConnectMessage, ConnectState, GetInfo, ListAction,
  ListMessage, MediaMessage, MediaType, MessageType, RequestMessage, RequestMessageData,
  ResponseMessage, ResponseMessageData, ResponseMessageData::Media, SignalMessage, UpdateName,
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use wasm_bindgen_futures::spawn_local;

use crate::{
  store::User,
  utils::{get_link, query_selector, Link, RTCLink, Request},
};

#[derive(Debug)]
pub struct Client {
  pub user: Rc<RefCell<User>>,
  links: Rc<RefCell<HashMap<String, RTCLink>>>,
  link: &'static mut Link,
}

impl Client {
  pub fn new(user: User) -> Self {
    let link = get_link().unwrap();
    let client = Client {
      user: Rc::new(RefCell::new(user)),
      links: Rc::new(RefCell::new(HashMap::new())),
      link,
    };
    client.watch_message();
    client
  }

  fn watch_message(&self) {
    let mut receiver = self.link.receiver();
    let links = self.links.clone();
    let sender = self.link.sender();
    let user = self.user.clone();
    spawn_local(async move {
      while let Ok(msg) = receiver.recv().await {
        log!("receiver message connect", &msg);
        if let Ok(origin_message) = serde_json::from_str::<ResponseMessage>(&msg) {
          let ResponseMessage {
            message,
            message_type,
            session_id,
          } = &origin_message;
          if let ResponseMessageData::Signal(message) = message {
            let SignalMessage { from, .. } = message;
            let links = links.borrow();
            let link = links.get(from);
            if link.is_some() {
              link.unwrap().parse_signal(origin_message).await;
            }
            continue;
          }

          if *message_type == MessageType::Response {
            continue;
          }
          let uuid = &user.borrow().uuid;
          if let ResponseMessageData::Connect(message) = message {
            let ConnectMessage { from, .. } = &message;
            let link = RTCLink::new(uuid.clone(), from.to_string()).unwrap();
            let dom = query_selector(".local-stream");
            let _ = link.set_local_user_media(dom).await;
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
    self.user.borrow().clone()
  }

  pub fn set_name(&mut self, name: String) {
    let uuid = self.user.borrow().uuid.clone();
    let user = User { uuid, name };
    *self.user.borrow_mut() = user;
  }
  pub fn set_user(&mut self, client: message::Client) {
    *self.user.borrow_mut() = client.into();
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

  fn extract_user(&self) -> (String, String) {
    let user = self.user.borrow().clone();
    (user.uuid, user.name)
  }

  pub async fn request_media(&mut self, to: String, media_type: MediaType) {
    let (uuid, name) = self.extract_user();
    let message = RequestMessageData::Media(MediaMessage {
      from: uuid,
      from_name: name,
      to: to.clone(),
      media_type,
      confirm: None,
    });
    let request = Request::new(self.link.sender(), self.link.receiver());
    let futures = request.feature();
    request.request(message);
    match futures.await {
      Ok(message) => {
        if let Media(MediaMessage { from, confirm, .. }) = message {
          if confirm.is_some_and(|x| x) {
            self.request_connect(from.clone()).await;
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
    let (uuid, name) = self.extract_user();
    let message = RequestMessageData::Media(MediaMessage {
      from: uuid,
      from_name: name,
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
    let message = RequestMessageData::Connect(ConnectMessage {
      from,
      to,
      state: ConnectState::CONNECTING,
    });
    Client::send_static(sender, message, MessageType::Response, session_id).await;
  }

  pub async fn request_connect(&mut self, to: String) {
    let (uuid, ..) = self.extract_user();
    let id = uuid.clone();
    let remote_id = to.clone();
    log!("request", to.clone(), id.clone());
    let link = RTCLink::new(id, remote_id).unwrap();
    let message = RequestMessageData::Connect(ConnectMessage {
      from: uuid,
      to: to.clone(),
      state: ConnectState::CONNECTING,
    });
    let request = Request::new(self.link.sender(), self.link.receiver());
    let futures = request.feature();
    request.request(message);
    match futures.await {
      Ok(_) => {
        // let dom = query_selector(".local-stream");
        // let _ = link.set_local_user_media(dom).await;
        // let _ = link.send_offer().await;
        log!("connect before");
        let _ = link.connect().await;
        log!("connect after");
        self.links.borrow_mut().insert(to.to_string(), link);
      }
      Err(err) => {
        log!("error", format!("{:?}", err));
      }
    }
  }
}
