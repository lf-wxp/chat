use std::{cell::RefCell, rc::Rc};

use crate::{BareChannel, Channel, ConnectMessage, RequestMessage, ResponseMessage};

pub struct ConnectChannel<T: Channel> {
  inner: BareChannel<T>,
}

impl<T: Channel> ConnectChannel<T> {
  pub fn new(channel: Rc<RefCell<T>>) -> Self {
    ConnectChannel {
      inner: BareChannel::new(channel),
    }
  }

  pub fn send_message(&mut self, message: ConnectMessage) {
    let action = RequestMessage::Connect(message);
    self.inner.send_message(action);
  }

  pub fn set_response_message(&self, callback: Box<dyn Fn(ConnectMessage)>) {
    let onmessage = Box::new(move |msg: ResponseMessage| {
      if let ResponseMessage::Connect(message) = msg {
        callback(message);
      }
    });
    self.inner.set_response_message(onmessage);
  }
}
