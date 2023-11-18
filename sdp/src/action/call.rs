use message::CallMessage;

use super::{UnicastExecute, UnicastMessage};

impl UnicastExecute for CallMessage {}
impl UnicastMessage for CallMessage {
  fn get_to(&self) -> String {
    self.to.clone()
  }
  fn get_message(&self) -> String {
    serde_json::to_string(&self.clone()).unwrap()
  }
}
