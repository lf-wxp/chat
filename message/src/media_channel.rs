use std::{cell::RefCell, rc::Rc};

use crate::{BareChannel, Channel, MediaMessage, RequestMessage, ResponseMessage};

pub struct MediaChannel<T: Channel> {
  inner: BareChannel<T>,
}

impl<T: Channel> MediaChannel<T> {
  pub fn new(channel: Rc<RefCell<T>>) -> Self {
    MediaChannel {
      inner: BareChannel::new(channel),
    }
  }

  pub fn send_message(&mut self, message: MediaMessage) {
    let action = RequestMessage::Media(message);
    self.inner.send_message(action);
  }

  pub fn set_response_message(&self, callback: Box<dyn Fn(MediaMessage)>) {
    let onmessage = Box::new(move |msg: ResponseMessage| {
      if let ResponseMessage::Media(message) = msg {
        callback(message);
      }
    });
    self.inner.set_response_message(onmessage);
  }
}
