use async_broadcast::Receiver;
use futures::{ready, select, Future, FutureExt, StreamExt};
use gloo_console::log;
use gloo_timers::future::TimeoutFuture;
use message::{ConnectMessage, ConnectState, ResponseMessage, ResponseMessageData};
use std::{
  pin::Pin,
  task::{Context, Poll},
};
use wasm_bindgen::JsValue;

#[derive(Debug, Clone)]
pub enum ConnectError {
  Timeout,
  Error,
}

#[derive(Debug, Clone)]
pub struct Connect {
  remote_id: String,
  receiver: Receiver<String>,
}

impl Connect {
  pub fn new(remote_id: String, receiver: Receiver<String>) -> Self {
    Connect {
      remote_id,
      receiver,
    }
  }
  pub async fn connect(
    self,
    send_future: impl Future<Output = Result<(), JsValue>>,
  ) -> Result<(), ConnectError> {
    let receiver = self.receiver.clone();
    let connect_future = ConnectFuture::new(self.remote_id, self.receiver);
    let timeout_future = TimeoutFuture::new(30 * 1000);
    let _ = send_future.await;
    async move {
      select! {
        data = connect_future.fuse() => {
          // receiver.close();
          data
        },
        _ = timeout_future.fuse() => {
          receiver.close();
          Err(ConnectError::Timeout)
        },
      }
    }
    .await
  }
}

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
  type Output = Result<(), ConnectError>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let this = self.get_mut();
    let msg = ready!(this.receiver.poll_next_unpin(cx));
    log!("connect poll", format!("{:?}", &msg));
    if let Some(msg) = msg {
      if let Ok(ResponseMessage {
        message: ResponseMessageData::Connect(ConnectMessage { state, from, .. }),
        ..
      }) = serde_json::from_str::<ResponseMessage>(&msg)
      {
        if from == this.remote_id {
          match state {
            ConnectState::Connected => {
              return Poll::Ready(Ok(()));
            }
            ConnectState::Failed => {
              return Poll::Ready(Err(ConnectError::Error));
            }
            _ => {}
          };
        }
      }
    }
    cx.waker().wake_by_ref();
    Poll::Pending
  }
}
