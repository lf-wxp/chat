use async_broadcast::{broadcast, Receiver, Sender};
use futures::{SinkExt, StreamExt};
use gloo_console::log;
use gloo_net::websocket::{futures::WebSocket, Message};
use wasm_bindgen_futures::spawn_local;

use crate::utils::SDP_SERVER;

#[derive(Debug)]
pub struct Link {
  sender: Sender<String>,
  receiver: Receiver<String>,
  read_sender: Sender<String>,
}

impl Link {
  pub fn new() -> Self {
    let ws = WebSocket::open(SDP_SERVER).unwrap();
    let (mut write, mut read) = ws.split();
    let (write_sender, write_receiver) = broadcast(10);
    let (read_sender, read_receiver) = broadcast(10);
    let sender_clone = read_sender.clone();
    spawn_local(async move {
      while let Some(msg) = read.next().await {
        match msg {
          Ok(msg) => {
            if let Message::Text(msg) = msg {
              log!("broadcast msg receive from sdp", &msg);
              let _ = sender_clone.broadcast_direct(msg.clone()).await;
              log!("broadcast msg receive send", &msg);
            }
          }
          Err(_) => todo!(),
        }
      }
      log!("WebSocket Closed")
    });
    let mut receiver_clone = write_receiver.clone();
    spawn_local(async move {
      while let Ok(msg) = receiver_clone.recv().await {
        log!("client before", &msg);
        let _ = write.send(Message::Text(msg)).await;
        log!("client after");
      }
    });
    Link {
      sender: write_sender,
      receiver: read_receiver,
      read_sender,
    }
  }

  pub fn sender(&self) -> Sender<String> {
    self.sender.clone()
  }
  pub fn receiver(&self) -> Receiver<String> {
    self.read_sender.new_receiver()
  }
}
