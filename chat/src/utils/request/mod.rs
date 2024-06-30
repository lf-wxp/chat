pub mod future;

pub use future::*;

use async_broadcast::{Receiver, Sender};
use gloo_console::log;
use message::{MessageType, RequestMessage, RequestMessageData};
use nanoid::nanoid;
use wasm_bindgen_futures::spawn_local;

use RequestFuture;

pub struct Request {
  sender: Sender<String>,
  receiver: Receiver<String>,
  session_id: String,
}

impl Request {
  pub fn new(sender: Sender<String>,receiver: Receiver<String>) -> Self {
    let session_id = nanoid!();
    Request { sender, session_id, receiver }
  }

  pub fn feature(&mut self) -> RequestFuture {
    let receiver = self.receiver.clone();
    RequestFuture::new(self.session_id.clone(), receiver)
  }
  pub fn request(&mut self, message: RequestMessageData) {
    let message = serde_json::to_string(&RequestMessage {
      message,
      session_id: self.session_id.clone(),
      message_type: MessageType::Request,
    })
    .unwrap();
    let sender = self.sender.clone();
    spawn_local(async move {
      log!("send message before", message.clone());
      let _ = sender.broadcast_direct(message).await;
      log!("send message after");
    });
  }
}
