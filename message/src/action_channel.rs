use std::{cell::RefCell, rc::Rc};

use crate::{Action, ActionMessage, Channel, RequestMessage, ResponseMessage};

type Callback = Rc<RefCell<Option<Box<dyn Fn(ActionMessage)>>>>;

pub struct ActionChannel<T: Channel> {
  channel: Rc<RefCell<T>>,
  receive_message: Callback,
}

impl<T: Channel> ActionChannel<T> {
  pub fn new(channel: Rc<RefCell<T>>) -> Self {
    let mut signal_channel = ActionChannel {
      channel,
      receive_message: Rc::new(RefCell::new(None)),
    };
    signal_channel.bind_event();
    signal_channel
  }

  fn bind_event(&mut self) {
    let receive_clone = self.receive_message.clone();
    let onmessage = Box::new(move |msg: &str| {
      if let Ok(ResponseMessage::Action(message)) = serde_json::from_str::<ResponseMessage>(msg) {
        if let Some(callback) = receive_clone.borrow_mut().as_ref() {
          callback(message);
        }
      }
    });
    self.channel.borrow_mut().onmessage(onmessage);
  }

  pub fn send_message(&mut self, message: Action) {
    let action = &RequestMessage::Action(message);
    let message = serde_json::to_string(action).unwrap();
    self.channel.borrow_mut().send(&message);
  }

  pub fn set_receive_message(&self, onmessage: Box<dyn Fn(ActionMessage)>) {
    *self.receive_message.borrow_mut() = Some(onmessage);
  }
}
