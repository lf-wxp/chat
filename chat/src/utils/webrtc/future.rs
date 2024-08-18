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
    log!(
      "connect future",
      format!("{:?}", &msg,),
      this.remote_id.clone()
    );
    if let Some(msg) = msg {
      if let Ok(ResponseMessage {
        message: ResponseMessageData::Connect(ConnectMessage { state, from, .. }),
        ..
      }) = serde_json::from_str::<ResponseMessage>(&msg)
      {
        if from == this.remote_id {
          if let ConnectState::CONNECTED = state {
            return Poll::Ready(());
          }
        }
      }
    }
    cx.waker().wake_by_ref();
    Poll::Pending
  }
}
