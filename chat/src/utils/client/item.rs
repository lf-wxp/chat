use async_broadcast::{broadcast, Receiver, Sender};
use futures::Future;
use gloo_console::log;
use js_sys::ArrayBuffer;
use message::{
  self, Action, ActionMessage, ClientAction, ConnectMessage, ConnectState, GetInfo, ListAction,
  ListMessage, MediaMessage, MediaType, MessageType, RequestMessage, RequestMessageData,
  ResponseMessage, ResponseMessageData, ResponseMessageData::Media, SignalMessage, UpdateName,
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use wasm_bindgen_futures::spawn_local;

use crate::{
  model::ChannelMessage,
  store::User,
  utils::{get_link, get_window, query_selector, Link, RTCLink, Request, RequestError, RtcType},
};

#[derive(Debug)]
pub struct Client {
  pub user: Rc<RefCell<User>>,
  links: Rc<RefCell<HashMap<String, RTCLink>>>,
  link: &'static mut Link,
  sender: Sender<ArrayBuffer>,
  pub receiver: Receiver<ArrayBuffer>,
}

impl Client {
  pub fn new(user: User) -> Self {
    let link = get_link().unwrap();
    let (sender, receiver) = broadcast(20);
    let client = Client {
      user: Rc::new(RefCell::new(user)),
      links: Rc::new(RefCell::new(HashMap::new())),
      link,
      sender,
      receiver,
    };
    client.watch_message();
    client
  }

  fn watch_message(&self) {
    let mut receiver = self.link.receiver();
    let links = self.links.clone();
    let sender = self.link.sender();
    let channel_message_sender = self.sender.clone();
    let user = self.user.clone();
    spawn_local(async move {
      while let Ok(msg) = receiver.recv().await {
        if let Ok(origin_message) = bincode::deserialize::<ResponseMessage>(&msg) {
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
          Client::receive_connect_request(
            &uuid,
            session_id,
            links.clone(),
            message,
            sender.clone(),
            channel_message_sender.clone(),
          )
          .await;
        }
      }
    })
  }

  async fn send(&self, message: RequestMessageData, message_type: MessageType, session_id: String) {
    let sender = self.link.sender();
    Client::send_static(&sender, message, message_type, session_id).await;
  }

  async fn send_static(
    sender: &Sender<Vec<u8>>,
    message: RequestMessageData,
    message_type: MessageType,
    session_id: String,
  ) {
    let message = bincode::serialize(&RequestMessage {
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

  fn set_link(&self, remote_id: &str, rtc_type: RtcType) {
    let (uuid, ..) = self.extract_user();
    let links = self.links.clone();
    let sender = self.sender.clone();
    Client::set_link_static(links, &uuid, remote_id, rtc_type, sender);
  }

  pub fn is_link_ready(&self, to: &str) -> bool {
    self.links.borrow().get(to).map_or(false, |x| x.is_ready())
  }

  pub fn is_datachannel_ready(&self, to: &str) -> bool {
    self
      .links
      .borrow()
      .get(to)
      .map_or(false, |x| x.is_datachannel_ready())
  }

  pub fn send_message(&mut self, remote_id: String, message: ChannelMessage) {
    let links = self.links.borrow();
    let link = links.get(&remote_id);
    if let Some(link) = link {
      if link.is_ready() {
        link.send_message(message.into());
      }
    }
  }

  pub fn send_message_multi(&mut self, remote_ids: Vec<String>, message: ChannelMessage) {
    remote_ids.iter().for_each(|x| {
      self.send_message(x.to_string(), message.clone());
    });
  }

  pub async fn request_datachannel(&mut self, remote_id: String) {
    if self.is_datachannel_ready(&remote_id) {
      return;
    }
    let links = self.links.clone();
    match self.request_connect(&remote_id, None).await {
      Ok(_) => {
        self.set_link(&remote_id, RtcType::Caller);
        Client::set_link_datachannel(links, &remote_id);
      }
      Err(_) => todo!(),
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
    let links = self.links.clone();
    let request = Request::new(self.link.sender(), self.link.receiver());
    match request.request(message).await {
      Ok(message) => {
        if let Media(MediaMessage { from, confirm, .. }) = message {
          if confirm.is_some_and(|x| x) {
            match self.request_connect(&from, Some(media_type.clone())).await {
              Ok(_) => {
                self.set_link(&to, RtcType::Caller);
                Client::set_link_media(links, &to, &Some(media_type)).await;
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
    sender: &Sender<Vec<u8>>,
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

  pub fn has_link(links: Rc<RefCell<HashMap<String, RTCLink>>>, remote_id: &str) -> bool {
    links.borrow().get(remote_id).is_some()
  }

  pub fn set_link_static(
    links: Rc<RefCell<HashMap<String, RTCLink>>>,
    id: &str,
    remote_id: &str,
    rtc_type: RtcType,
    sender: Sender<ArrayBuffer>,
  ) {
    if links.borrow().get(remote_id).is_none() {
      let link = RTCLink::new(id.to_string(), remote_id.to_string(), sender, rtc_type).unwrap();
      links.borrow_mut().insert(remote_id.to_string(), link);
    }
  }

  pub async fn set_link_media(
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

  pub fn set_link_datachannel(links: Rc<RefCell<HashMap<String, RTCLink>>>, remote_id: &str) {
    if let Some(link) = links.borrow_mut().get_mut(remote_id) {
      link.create_datachannel();
    }
  }

  pub async fn receive_connect_request(
    uuid: &str,
    session_id: &str,
    links: Rc<RefCell<HashMap<String, RTCLink>>>,
    message: &ResponseMessageData,
    sender: Sender<Vec<u8>>,
    channel_message_sender: Sender<ArrayBuffer>,
  ) {
    if let ResponseMessageData::Connect(ConnectMessage {
      from,
      state,
      media_type,
      ..
    }) = message
    {
      if ConnectState::New != *state {
        return;
      }
      if !Client::has_link(links.clone(), from) {
        Client::set_link_static(
          links.clone(),
          uuid,
          from,
          RtcType::Callee,
          channel_message_sender,
        );
        Client::set_link_media(links.clone(), from, media_type).await;
      }
      Client::replay_request_connect(
        &sender,
        uuid.to_string(),
        from.clone(),
        session_id.to_string(),
      )
      .await;
      log!(
        "request connect before",
        get_window().performance().unwrap().now()
      );
    }
  }

  pub async fn request_connect(
    &mut self,
    to: &str,
    media_type: Option<MediaType>,
  ) -> Result<ResponseMessageData, RequestError> {
    let (uuid, ..) = self.extract_user();
    let message = RequestMessageData::Connect(ConnectMessage {
      from: uuid,
      to: to.to_string(),
      state: ConnectState::New,
      media_type,
    });
    let request = Request::new(self.link.sender(), self.link.receiver());
    request.request(message).await
  }
}
