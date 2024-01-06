use futures::{SinkExt, StreamExt};
use gloo_console::log;
use gloo_net::websocket::{futures::WebSocket, Message};
use wasm_bindgen_futures::spawn_local;
use async_broadcast::{broadcast, Receiver, Sender};

type MessageFn = Box<dyn Fn(Message)>;

pub struct Websocket {
  write_sender: Sender<Message>,
  write_receiver: Receiver<Message>,
  read_sender: Sender<Message>,
  read_receiver: Receiver<Message>,
}

impl Websocket {
  pub fn new(url: &str) -> Self {
    let (write_sender, write_receiver) = broadcast(10);
    let (read_sender, read_receiver) = broadcast(10);
    let mut client = Websocket {
      write_sender,
      write_receiver,
      read_sender,
      read_receiver,
    };
    client.setup(url);
    client
  }

  fn setup(&mut self, url: &str) {
    let ws = WebSocket::open(url).unwrap();
    let sender = self.read_sender.clone();
    let (mut write, mut read) = ws.split();
    spawn_local(async move {
      while let Some(msg) = read.next().await {
        match msg {
          Ok(msg) => {
            sender.send(msg);
          }
          Err(_) => todo!(),
        }
      }
      log!("WebSocket Closed")
    });

    let receiver = self.write_receiver.clone();
    spawn_local(async move {
      while let Ok(msg) = receiver.next().await {
        let _ = write.send(msg).await;
      }
    });
  }

  pub fn send(&self, message: String) {
    self.write_sender.send(Message::Text(message));
  }

  pub fn get_receiver(&self) -> Receiver<Message> {
    self.read_receiver
  }
}
