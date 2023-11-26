use message::{ConnectMessage, ResponseMessage};

use super::{UnicastExecute, UnicastMessage};

impl UnicastExecute for ConnectMessage {}
impl UnicastMessage for ConnectMessage {
  fn get_to(&self) -> String {
    self.to.clone()
  }
  fn get_message(&self) -> String {
    serde_json::to_string(&ResponseMessage::Connect(self.clone())).unwrap()
  }
}
