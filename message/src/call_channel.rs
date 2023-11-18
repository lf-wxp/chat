use std::{cell::RefCell, rc::Rc};

use crate::{Action, CallMessage, CallType, Channel, RequestMessage, ResponseMessage};

type Callback = Rc<RefCell<Option<Box<dyn Fn(CallMessage)>>>>;

pub struct CallChannel<T: Channel> {
  channel: Rc<RefCell<T>>,
  response_message: Callback,
}

impl<T: Channel> CallChannel<T> {
  pub fn new(channel: Rc<RefCell<T>>) -> Self {
    let mut call_channel = CallChannel {
      channel,
      response_message: Rc::new(RefCell::new(None)),
    };
    call_channel.bind_event();
    call_channel
  }

  fn bind_event(&mut self) {
    let receive_clone = self.response_message.clone();
    let onmessage = Box::new(move |msg: &str| {
      if let Ok(ResponseMessage::Call(message)) = serde_json::from_str::<ResponseMessage>(msg) {
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

  pub fn set_response_message(&self, onmessage: Box<dyn Fn(CallMessage)>) {
    *self.response_message.borrow_mut() = Some(onmessage);
  }

  pub fn call(&self, caller: String, callee: String) {
    let action = &RequestMessage::Call(CallMessage {
      from: caller,
      to: callee,
      call_type: CallType::Video,
      expired: None,
      confirm: None,
    });
    let message = serde_json::to_string(action).unwrap();
    self.channel.borrow_mut().send(&message);
  }
}
