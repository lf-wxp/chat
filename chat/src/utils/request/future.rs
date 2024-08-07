use std::{
  pin::Pin,
  task::{Context, Poll},
};
use async_broadcast::Receiver;
use futures::{ready, Future, StreamExt};
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
    let msg = ready!(this.receiver.poll_next_unpin(cx));
    if let Some(msg) = msg {
      match serde_json::from_str::<ResponseMessage>(&msg) {
        Ok(ResponseMessage {
          message,
          session_id,
          ..
        }) => {
          if session_id == this.session_id {
            return Poll::Ready(message);
          }
          return Poll::Pending;
        }
        Err(err) => {
          return Poll::Pending;
        }
      }
    }
    Poll::Pending
  }
}
