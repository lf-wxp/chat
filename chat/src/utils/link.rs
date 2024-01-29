use futures::{SinkExt, StreamExt};
use gloo_console::log;
use gloo_net::websocket::{futures::WebSocket, Message};
use message::{MessageType, RequestMessage, ResponseMessage, ResponseMessageData};
use postage::{
  broadcast::{self, Receiver, Sender},
  sink::Sink,
};
use wasm_bindgen_futures::spawn_local;

use crate::utils::SDP_SERVER;

pub struct Link {
  read_sender: Sender<String>,
  read_receiver: Receiver<String>,
  write_sender: Sender<String>,
  write_receiver: Receiver<String>,
}

impl Link {
  pub fn new() -> Self {
    // let ws = WebSocket::open(SDP_SERVER).unwrap();
    // let (mut write, mut read) = ws.split();
    let ws = broadcast::channel::<String>(0);
    let (mut write, mut read) = ws;
    let (read_sender, read_receiver) = broadcast::channel(0);
    let (write_sender, write_receiver) = broadcast::channel(0);
    let mut sender = read_sender.clone();
    spawn_local(async move {
      while let Some(msg) = read.next().await {
        log!("broadcast msg receive", &msg);
        match serde_json::from_str::<RequestMessage>(&msg) {
          Ok(msg) => {
            let RequestMessage { session_id, .. } = msg;
            let msg = serde_json::to_string(&ResponseMessage {
              session_id,
              message: ResponseMessageData::Action(message::ActionMessage::Success),
              message_type: MessageType::Response,
            }).unwrap();
            let _ = sender.send(msg.clone()).await;
          }
          Err(_) => todo!(),
        }
        // match msg {
        //   Ok(msg) => {
        //     if let Message::Text(msg) = msg {
        //       log!("broadcast msg receive", &msg);
        //       let _ = sender.blocking_send(msg.clone());
        //     }
        //   }
        //   Err(_) => todo!(),
        // }
      }
      log!("WebSocket Closed")
    });
    let mut receiver = write_receiver.clone();
    log!("link init");
    spawn_local(async move {
      while let Some(msg) = receiver.next().await {
        log!("broadcast msg", &msg);
        // let _ = write.send(Message::Text(msg)).await;
        let _ = write.send(msg).await;
      }
    });
    Link {
      read_sender,
      read_receiver,
      write_sender,
      write_receiver,
    }
  }

  pub fn receiver(&self) -> Receiver<String> {
    // self.read_receiver.clone()
    self.read_sender.subscribe()
  }

  pub fn sender(&self) -> Sender<String> {
    self.write_sender.clone()
  }
}
