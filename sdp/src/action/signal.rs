use message::{SignalMessage, ResponseMessage};

use super::{UnicastExecute, UnicastMessage};

impl UnicastExecute for SignalMessage {}
impl UnicastMessage for SignalMessage {
  fn get_to(&self) -> String {
    self.to.clone()
  }
  fn get_message(&self) -> String {
    serde_json::to_string(&ResponseMessage::Signal(self.clone())).unwrap()
  }
}
