use std::{cell::RefCell, rc::Rc};

use crate::{Action, ActionMessage, BareChannel, Channel, RequestMessage, ResponseMessage};

pub struct ActionChannel<T: Channel> {
  inner: BareChannel<T>,
}

impl<T: Channel> ActionChannel<T> {
  pub fn new(channel: Rc<RefCell<T>>) -> Self {
    ActionChannel {
      inner: BareChannel::new(channel),
    }
  }

  pub fn send_message(&mut self, message: Action) {
    let action = RequestMessage::Action(message);
    self.inner.send_message(action);
  }

  pub fn set_response_message(&self, callback: Box<dyn Fn(ActionMessage)>) {
    let onmessage = Box::new(move |msg: ResponseMessage| {
      if let ResponseMessage::Action(message) = msg {
        callback(message);
      }
    });
    self.inner.set_response_message(onmessage);
  }
}
