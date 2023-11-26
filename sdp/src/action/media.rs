use message::{MediaMessage, ResponseMessage};

use super::{UnicastExecute, UnicastMessage};

impl UnicastExecute for MediaMessage {}
impl UnicastMessage for MediaMessage {
  fn get_to(&self) -> String {
    self.to.clone()
  }
  fn get_message(&self) -> String {
    serde_json::to_string(&ResponseMessage::Media(self.clone())).unwrap()
  }
}
