use gloo_console::log;
use message::{MessageType, RequestMessage, RequestMessageData};
use nanoid::nanoid;
use postage::{
  broadcast::{Receiver, Sender},
  sink::Sink,
};

use super::RequestFuture;

pub struct Request {
  sender: Sender<String>,
  receiver: Receiver<String>,
}

impl Request {
  pub fn new(sender: Sender<String>, receiver: Receiver<String>) -> Self {
    Request { receiver, sender }
  }

  pub fn request(&mut self, message: RequestMessageData) -> RequestFuture {
    let receiver = self.receiver.clone();
    let session_id = nanoid!();
    log!("session_id", session_id.clone());
    let message = serde_json::to_string(&RequestMessage {
      message,
      session_id: session_id.clone(),
      message_type: MessageType::Request,
    })
    .unwrap();
    let mut sender = self.sender.clone();
    let _ = sender.blocking_send(message);
    RequestFuture::new(session_id, receiver)
  }
}
