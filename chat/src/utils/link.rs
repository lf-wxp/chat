use futures::{SinkExt, StreamExt};
use gloo_console::log;
use gloo_net::websocket::{futures::WebSocket, Message};
use message::{MessageType, RequestMessage, ResponseMessage, ResponseMessageData};
use async_broadcast::{
  broadcast, Sender,Receiver
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
    let ws = WebSocket::open(SDP_SERVER).unwrap();
    let (mut write, mut read) = ws.split();
    let (read_sender, read_receiver) = broadcast(2);
    let (write_sender, write_receiver) = broadcast(2);
    let sender = read_sender.clone();
    spawn_local(async move {
      while let Some(msg) = read.next().await {
        match msg {
          Ok(msg) => {
            if let Message::Text(msg) = msg {
              log!("broadcast msg receive", &msg);
              let _ = sender.broadcast_direct(msg.clone()).await;
            }
          }
          Err(_) => todo!(),
        }
      }
      log!("WebSocket Closed")
    });
    let mut receiver = write_receiver.clone();
    log!("link init");
    spawn_local(async move {
      while let Some(msg) = receiver.next().await {
        log!("broadcast msg", &msg);
        let _ = write.send(Message::Text(msg)).await;
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
    self.read_receiver.clone()
  }

  pub fn sender(&self) -> Sender<String> {
    self.write_sender.clone()
  }
}
