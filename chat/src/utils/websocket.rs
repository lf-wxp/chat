use async_broadcast::{broadcast, Receiver, Sender};
use futures::{SinkExt, StreamExt};
use gloo_console::log;
use gloo_net::websocket::{futures::WebSocket, Message};
use wasm_bindgen_futures::spawn_local;

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
            sender.broadcast(msg);
          }
          Err(_) => todo!(),
        }
      }
      log!("WebSocket Closed")
    });

    let receiver = self.write_receiver.clone();
    spawn_local(async move {
      while let Some(msg) = receiver.next().await {
        let _ = write.send(msg).await;
      }
    });
  }

  pub fn send(&self, message: String) {
    self.write_sender.broadcast(Message::Text(message));
  }

  pub fn get_channel(&self) -> (Sender<Message>, Receiver<Message>) {
    (self.write_sender, self.read_receiver)
  }

  pub fn get_channel_fn(&self) -> (Box<dyn Fn(String)>, Receiver<Message>) {
    (
      Box::new(|message: String| { self.write_sender.broadcast(Message::Text(message)); }),
      self.read_receiver,
    )
  }
}
