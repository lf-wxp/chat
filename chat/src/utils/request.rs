use std::{
  pin::Pin, task::{Context, Poll}, time::Duration
};

use futures::{ready, Future, StreamExt};
use gloo_console::log;
use gloo_timers::future::sleep;
use message::{
  MessageType, RequestMessage, RequestMessageData, ResponseMessage, ResponseMessageData,
};
use nanoid::nanoid;
use postage::{
  broadcast::{Receiver, Sender},
  sink::Sink,
};
use wasm_bindgen_futures::spawn_local;

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
    let message = serde_json::to_string(&RequestMessage {
      message,
      session_id: session_id.clone(),
      message_type: MessageType::Request,
    })
    .unwrap();
    let mut sender = self.sender.clone();
    spawn_local(async move {
      sleep(Duration::from_secs(3)).await;
      let _ = sender.blocking_send(message);
    });
    RequestFuture::new(session_id, receiver)
  }
}

pub struct RequestFuture {
  session_id: String,
  receiver: Receiver<String>,
}

impl RequestFuture {
  pub fn new(session_id: String, receiver: Receiver<String>) -> Self {
    RequestFuture {
      session_id,
      receiver,
    }
  }
}

impl Future for RequestFuture {
  type Output = ResponseMessageData;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let this = self.get_mut();
    log!("poll before");
    let msg = ready!(this.receiver.poll_next_unpin(cx));
    if let Some(msg) = msg {
      match serde_json::from_str::<ResponseMessage>(&msg) {
        Ok(ResponseMessage {
          message,
          session_id,
          ..
        }) => {
          log!("poll xxx", format!("{:?}", &msg));
          if session_id == this.session_id {
            log!("poll  after ", format!("{:?}", &message));
            return Poll::Ready(message);
          }
          return Poll::Pending;
        }
        Err(_) => return Poll::Pending,
      }
    }
    Poll::Pending
  }
}
