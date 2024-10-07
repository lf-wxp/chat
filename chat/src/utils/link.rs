use async_broadcast::{broadcast, Receiver, Sender};
use futures::{SinkExt, StreamExt};
use gloo_console::log;
use gloo_net::websocket::{futures::WebSocket, Message};
use wasm_bindgen_futures::spawn_local;

use crate::utils::SDP_SERVER;

#[derive(Debug)]
pub struct Link {
  sender: Sender<Vec<u8>>,
  pub receiver: Receiver<Vec<u8>>,
  read_sender: Sender<Vec<u8>>,
}

impl Link {
  pub fn new() -> Self {
    let ws = WebSocket::open(SDP_SERVER).unwrap();
    let (mut write, mut read) = ws.split();
    let (write_sender, write_receiver) = broadcast::<Vec<u8>>(20);
    let (read_sender, read_receiver) = broadcast::<Vec<u8>>(20);
    let sender_clone = read_sender.clone();
    spawn_local(async move {
      while let Some(msg) = read.next().await {
        match msg {
          Ok(msg) => {
            if let Message::Bytes(msg) = msg {
              let _ = sender_clone.broadcast_direct(msg.clone()).await;
            }
          }
          Err(msg) => {
            log!("error is ", format!("{:?}", msg));
          },
        }
      }
    });
    let mut receiver_clone = write_receiver.clone();
    spawn_local(async move {
      while let Ok(msg) = receiver_clone.recv().await {
        let _ = write.send(Message::Bytes(msg)).await;
      }
    });
    Link {
      sender: write_sender,
      receiver: read_receiver,
      read_sender,
    }
  }

  pub fn sender(&self) -> Sender<Vec<u8>> {
    self.sender.clone()
  }
  pub fn receiver(&self) -> Receiver<Vec<u8>> {
    self.read_sender.new_receiver()
  }
}
