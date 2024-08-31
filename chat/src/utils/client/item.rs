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
  utils::{get_link, get_window, query_selector, Link, RTCLink, Request, RequestError},
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
          let uuid = user.borrow().uuid.clone();
          if let ResponseMessageData::Connect(ConnectMessage {
            from,
            state,
            media_type,
            ..
          }) = message
          {
            log!("connect", format!("{:?}", *message));
            if ConnectState::New != *state {
              return;
            }
            Client::set_link_static(links.clone(), &uuid, from);
            Client::set_link_media_static(links.clone(), from, media_type).await;
            Client::replay_request_connect(&sender, uuid, from.clone(), session_id.clone()).await;
            log!(
              "request connect before",
              get_window().performance().unwrap().now()
            );
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
    if let Ok(ResponseMessageData::Action(ActionMessage::Client(info))) =
      request.request(message).await
    {
      self.set_user(info.clone());
      return Some(info);
    }
    None
  }

  pub async fn get_user_list(&mut self) -> Option<ListMessage> {
    let message = RequestMessageData::Action(Action::List(ListAction));
    let request = Request::new(self.link.sender(), self.link.receiver());
    if let Ok(ResponseMessageData::Action(ActionMessage::ListMessage(list_message))) =
      request.request(message).await
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
  ) -> impl Future<Output = Result<ResponseMessageData, RequestError>> {
    let message =
      RequestMessageData::Action(Action::Client(ClientAction::UpdateName(UpdateName {
        name,
      })));
    let request = Request::new(self.link.sender(), self.link.receiver());
    request.request(message)
  }

  fn extract_user(&self) -> (String, String) {
    let user = self.user.borrow().clone();
    (user.uuid, user.name)
  }

  fn set_link(&self, remote_id: &str) {
    let (uuid, ..) = self.extract_user();
    let links = self.links.clone();
    Client::set_link_static(links, &uuid, remote_id);
  }
  async fn set_link_media(&self, remote_id: &str, media_type: &Option<MediaType>) {
    let links = self.links.clone();
    Client::set_link_media_static(links, remote_id, media_type).await;
  }

  async fn link_connect(&self, remote_id: &str) {
    let links = self.links.borrow();
    let link = links.get(remote_id);
    if let Some(link) = link {
      if (link.connect().await).is_ok() {
        log!("connected");
      }
    }
  }

  pub async fn request_media(&mut self, to: String, media_type: MediaType) {
    let (uuid, name) = self.extract_user();
    let message = RequestMessageData::Media(MediaMessage {
      from: uuid,
      from_name: name,
      to: to.clone(),
      media_type: media_type.clone(),
      confirm: None,
    });
    let request = Request::new(self.link.sender(), self.link.receiver());
    match request.request(message).await {
      Ok(message) => {
        if let Media(MediaMessage { from, confirm, .. }) = message {
          if confirm.is_some_and(|x| x) {
            match self
              .request_connect(from.clone(), Some(media_type.clone()))
              .await
            {
              Ok(_) => {
                self.set_link(&to);
                self.set_link_media(&to, &Some(media_type)).await;
                self.link_connect(&to).await;
              }
              Err(_) => todo!(),
            }
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
    self
      .replay_request_media(message.clone(), true, session_id)
      .await;
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
      state: ConnectState::Checking,
      media_type: None,
    });
    Client::send_static(sender, message, MessageType::Response, session_id).await;
  }

  pub fn set_link_static(links: Rc<RefCell<HashMap<String, RTCLink>>>, id: &str, remote_id: &str) {
    if links.borrow().get(remote_id).is_none() {
      let link = RTCLink::new(id.to_string(), remote_id.to_string()).unwrap();
      links.borrow_mut().insert(remote_id.to_string(), link);
    }
  }

  pub async fn set_link_media_static(
    links: Rc<RefCell<HashMap<String, RTCLink>>>,
    remote_id: &str,
    media_type: &Option<MediaType>,
  ) {
    let links = links.borrow();
    let link = links.get(remote_id);
    let dom = query_selector(".local-stream");
    if let Some(link) = link {
      let _ = link.set_media(dom, media_type).await;
      log!("set link media");
    }
  }

  pub async fn request_connect(
    &mut self,
    to: String,
    media_type: Option<MediaType>,
  ) -> Result<ResponseMessageData, RequestError> {
    let (uuid, ..) = self.extract_user();
    let message = RequestMessageData::Connect(ConnectMessage {
      from: uuid,
      to: to.clone(),
      state: ConnectState::New,
      media_type,
    });
    let request = Request::new(self.link.sender(), self.link.receiver());
    request.request(message).await
  }
}
