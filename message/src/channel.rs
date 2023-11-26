use std::{cell::RefCell, rc::Rc};

use gloo_console::log;

use crate::{RequestMessage, ResponseMessage};

pub trait Channel {
  fn send(&self, _message: &str) {}
  fn onmessage(&mut self, _callback: Box<dyn Fn(&str)>) {}
}

type Callback = Rc<RefCell<Option<Box<dyn Fn(ResponseMessage)>>>>;

pub struct BareChannel<T: Channel> {
  channel: Rc<RefCell<T>>,
  response_message: Callback,
}

impl<T: Channel> BareChannel<T> {
  pub fn new(channel: Rc<RefCell<T>>) -> Self {
    let mut bare_channel = BareChannel {
      channel,
      response_message: Rc::new(RefCell::new(None)),
    };
    bare_channel.bind_event();
    bare_channel
  }

  fn bind_event(&mut self) {
    let receive_clone = self.response_message.clone();
    let onmessage = Box::new(move |msg: &str| {
        log!("receive msg");
      if let Some(callback) = receive_clone.borrow_mut().as_ref() {
        if let Ok(message) = serde_json::from_str::<ResponseMessage>(msg) {
          callback(message);
        }
      }
    });
    self.channel.borrow_mut().onmessage(onmessage);
  }

  pub fn send_message(&mut self, message: RequestMessage) {
    let message = serde_json::to_string(&message).unwrap();
    self.channel.borrow_mut().send(&message);
  }

  pub fn set_response_message(&self, onmessage: Box<dyn Fn(ResponseMessage)>) {
    *self.response_message.borrow_mut() = Some(onmessage);
  }
}
