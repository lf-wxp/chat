use async_broadcast::{Receiver, Sender};
use gloo_console::log;
use message::{MessageType, RequestMessage, RequestMessageData};
use nanoid::nanoid;
use wasm_bindgen_futures::spawn_local;

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
    let sender = self.sender.clone();
    spawn_local(async move {
      let _ = sender.broadcast_direct(message).await;
    });
    RequestFuture::new(session_id, receiver)
  }
}
