pub mod future;

pub use future::*;

use async_broadcast::{Receiver, Sender};
use futures::{select, Future, FutureExt};
use gloo_console::log;
use gloo_timers::future::TimeoutFuture;
use message::{MessageType, RequestMessage, RequestMessageData, ResponseMessageData};
use nanoid::nanoid;
use wasm_bindgen_futures::spawn_local;

use RequestFuture;

#[derive(Debug, Clone)]
pub struct Request {
  sender: Sender<String>,
  receiver: Receiver<String>,
  session_id: String,
}

impl Request {
  pub fn new(sender: Sender<String>, receiver: Receiver<String>) -> Self {
    let session_id = nanoid!();
    Request {
      sender,
      session_id,
      receiver,
    }
  }

  pub fn feature(&self) -> impl Future<Output = Result<ResponseMessageData, ()>> {
    let receiver = self.receiver.clone();
    let timeout_future = TimeoutFuture::new(30 * 1000);
    let session_id = self.session_id.clone();
    async move {
      select! {
        data = RequestFuture::new(session_id, receiver).fuse() => Ok(data),
        _ = timeout_future.fuse() => Err(()),
      }
    }
  }
  pub fn request(&self, message: RequestMessageData) {
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
