use message::{MessageType, ResponseMessage, ResponseMessageData, SignalMessage};

use super::{UnicastExecute, UnicastMessage};

impl UnicastExecute for SignalMessage {}
impl UnicastMessage for SignalMessage {
  fn get_to(&self) -> String {
    self.to.clone()
  }
  fn get_message(&self, session_id: String, message_type: MessageType) -> String {
    serde_json::to_string(&ResponseMessage {
      session_id,
      message: ResponseMessageData::Signal(self.clone()),
      message_type,
    })
    .unwrap()
  }
}
