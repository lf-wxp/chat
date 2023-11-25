use std::{cell::RefCell, rc::Rc};

use futures::{SinkExt, StreamExt};
use futures_channel::mpsc::{self, UnboundedSender};
use gloo_console::log;
use gloo_net::websocket::{futures::WebSocket, Message};
use gloo_utils::errors::JsError;
use message::Channel;
use wasm_bindgen_futures::spawn_local;

type MessageFn = Box<dyn Fn(Message)>;

pub struct Websocket {
  onmessage: Rc<RefCell<Vec<MessageFn>>>,
  sender_ws: Option<UnboundedSender<String>>,
  sender_callback: Option<UnboundedSender<MessageFn>>,
}

impl Websocket {
  pub fn new(url: &str) -> Result<Rc<RefCell<Self>>, JsError> {
    let client = Rc::new(RefCell::new(Websocket {
      onmessage: Rc::new(RefCell::new(vec![])),
      sender_ws: None,
      sender_callback: None,
    }));
    client.borrow_mut().setup(url);
    Ok(client)
  }

  fn setup(&mut self, url: &str) {
    let ws = WebSocket::open(url).unwrap();
    let message_callback = self.onmessage.clone();
    let (sender_ws, mut receiver_ws) = mpsc::unbounded();
    let (sender_callback, mut receiver_callback) = mpsc::unbounded();
    self.sender_ws = Some(sender_ws);
    self.sender_callback = Some(sender_callback);
    let (mut write, mut read) = ws.split();
    spawn_local(async move {
      while let Some(msg) = read.next().await {
        match msg {
          Ok(msg) => {
            message_callback
              .borrow()
              .iter()
              .for_each(|callback| callback(msg.clone()));
          }
          Err(_) => todo!(),
        }
      }
      log!("WebSocket Closed")
    });

    spawn_local(async move {
      while let Some(msg) = receiver_ws.next().await {
        let _ = write.send(Message::Text(msg)).await;
      }
    });

    let message_callback = self.onmessage.clone();
    spawn_local(async move {
      while let Some(callback) = receiver_callback.next().await {
        message_callback.borrow_mut().push(callback);
      }
    });
  }

  pub fn set_onmessage(&self, callback: Box<dyn Fn(Message)>) {
    if let Some(sender) = &self.sender_callback {
      let _ = sender.unbounded_send(callback);
    }
  }

  pub fn send_message(&self, message: String) {
    if let Some(sender) = &self.sender_ws {
      let _ = sender.unbounded_send(message);
    }
  }
}

impl Channel for Websocket {
  fn send(&self, message: &str) {
    let message = String::from(message);
    self.send_message(message);
  }

  fn onmessage(&mut self, callback: Box<dyn Fn(&str)>) {
    let onmessage = Box::new(move |message: Message| {
      if let Message::Text(msg) = message {
        callback(&msg);
      }
    });
    self.set_onmessage(onmessage);
  }
}
