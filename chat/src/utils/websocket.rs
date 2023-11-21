use std::{cell::RefCell, rc::Rc};

use futures::{SinkExt, StreamExt};
use futures_channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use gloo_console::log;
use gloo_net::websocket::{futures::WebSocket, Message};
use gloo_utils::errors::JsError;
use message::Channel;
use wasm_bindgen_futures::spawn_local;

pub struct Websocket {
  onmessage: Rc<RefCell<Vec<Box<dyn Fn(Message)>>>>,
  sender_ws: UnboundedSender<String>,
  receiver_ws: Rc<RefCell<UnboundedReceiver<String>>>,
  sender_callback: Rc<RefCell<UnboundedSender<Box<dyn Fn(Message)>>>>,
  receiver_callback: Rc<RefCell<UnboundedReceiver<Box<dyn Fn(Message)>>>>,
}

impl Websocket {
  pub fn new(url: &str) -> Result<Rc<RefCell<Self>>, JsError> {
    let (sender_ws, receiver_ws) = mpsc::unbounded();
    let (sender_callback, receiver_callback) = mpsc::unbounded();
    let client = Rc::new(RefCell::new(Websocket {
      onmessage: Rc::new(RefCell::new(vec![])),
      sender_ws,
      receiver_ws: Rc::new(RefCell::new(receiver_ws)),
      sender_callback: Rc::new(RefCell::new(sender_callback)),
      receiver_callback: Rc::new(RefCell::new(receiver_callback)),
    }));
    client.borrow().setup(url);
    Ok(client)
  }

  fn setup(&self, url: &str) {
    let ws = WebSocket::open(url).unwrap();
    let message_callback = self.onmessage.clone();
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

    let receiver = self.receiver_ws.clone();
    spawn_local(async move {
      while let Some(msg) = receiver.borrow_mut().next().await {
        let _ = write.send(Message::Text(msg)).await;
      }
    });

    let receiver = self.receiver_callback.clone();
    let message_callback = self.onmessage.clone();
    spawn_local(async move {
      while let Some(callback) = receiver.borrow_mut().next().await {
        message_callback.borrow_mut().push(callback);
      }
    });
  }

  pub fn set_onmessage(&self, callback: Box<dyn Fn(Message)>) {
    self.sender_callback.borrow_mut().unbounded_send(callback);
  }

  pub fn send_message(&self, message: String) {
    let _ = self.sender_ws.unbounded_send(message);
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
