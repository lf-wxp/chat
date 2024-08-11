use async_broadcast::Receiver;
use futures::{ready, Future, StreamExt};
use gloo_console::log;
use message::{ConnectMessage, ConnectState, ResponseMessage, ResponseMessageData};
use std::{
  pin::Pin,
  task::{Context, Poll},
};

pub struct ConnectFuture {
  remote_id: String,
  receiver: Receiver<String>,
}

impl ConnectFuture {
  pub fn new(remote_id: String, receiver: Receiver<String>) -> Self {
    ConnectFuture {
      remote_id,
      receiver,
    }
  }
}

impl Future for ConnectFuture {
  type Output = ();

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let this = self.get_mut();
    let msg = ready!(this.receiver.poll_next_unpin(cx));
    log!("connect future", format!("{:?}", &msg,), this.remote_id.clone());
    if let Some(msg) = msg {
      match serde_json::from_str::<ResponseMessage>(&msg) {
        Ok(ResponseMessage { message, .. }) => {
          if let ResponseMessageData::Connect(ConnectMessage { state, from, .. }) = message {
            if from != this.remote_id {
              return Poll::Pending;
            }
            match state {
              ConnectState::CONNECTED => {
                return Poll::Ready(());
              }
              _ => {
                return Poll::Pending;
              }
            }
          }
          return Poll::Pending;
        }
        Err(_) => return Poll::Pending,
      }
    }
    Poll::Pending
  }
}
