use message::{ConnectMessage, MessageType, ResponseMessage, ResponseMessageData};

use super::{UnicastExecute, UnicastMessage};

impl UnicastExecute for ConnectMessage {}
impl UnicastMessage for ConnectMessage {
  fn get_to(&self) -> String {
    self.to.clone()
  }
  fn get_message(&self, session_id: String, message_type: MessageType) -> String {
    serde_json::to_string(&ResponseMessage {
      session_id,
      message: ResponseMessageData::Connect(self.clone()),
      message_type,
    })
    .unwrap()
  }
}
