use std::{
  pin::Pin,
  task::{Context, Poll},
};

use futures::{ready, Future, StreamExt};
use gloo_console::log;
use message::{MediaMessage, ResponseMessage};
use postage::broadcast::Receiver;

pub struct RequestMedia {
  receiver: Receiver<String>,
}

impl RequestMedia {
  pub fn new(receiver: Receiver<String>) -> Self {
    RequestMedia { receiver }
  }
}

impl Future for RequestMedia {
  type Output = MediaMessage;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let this = self.get_mut();
    log!("poll before");
    let msg = ready!(this.receiver.poll_next_unpin(cx));
    if let Some(msg) = msg {
      match serde_json::from_str::<ResponseMessage>(&msg) {
        Ok(msg) => {
          log!("poll xxx", format!("{:?}", &msg));
          if let ResponseMessage::Media(message) = msg {
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
