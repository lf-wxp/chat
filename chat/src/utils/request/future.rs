use std::{
  pin::Pin,
  task::{Context, Poll},
};

use async_broadcast::Receiver;
use futures::{ready, Future, StreamExt};
use gloo_console::log;
use message::{ResponseMessage, ResponseMessageData};

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
    log!("poll before", &this.session_id);
    let msg = ready!(this.receiver.poll_next_unpin(cx));
    if let Some(msg) = msg {
      log!("poll  after ", format!("{:?}", &msg));
      match serde_json::from_str::<ResponseMessage>(&msg) {
        Ok(ResponseMessage {
          message,
          session_id,
          ..
        }) => {
          log!(
            "poll xxx",
            &session_id,
            &this.session_id,
            format!("{:?}", &msg)
          );
          if session_id == this.session_id {
            log!("poll  after ", format!("{:?}", &message));
            return Poll::Ready(message);
          }
          return Poll::Pending;
        }
        Err(err) => {
          log!("poll error", format!("{:?}", err));
          return Poll::Pending;
        }
      }
    }
    Poll::Pending
  }
}
