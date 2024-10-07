use message::{ChatMessage, MessageType, ResponseMessage, ResponseMessageData };

use super::{UnicastExecute, UnicastMessage};

impl UnicastExecute for ChatMessage {}
impl UnicastMessage for ChatMessage {
  fn get_to(&self) -> String {
    self.to[0].clone()
  }
  fn get_message(&self, session_id: String, message_type: MessageType) -> Vec<u8> {
    bincode::serialize(&ResponseMessage {
      session_id,
      message: ResponseMessageData::Chat(self.clone()),
      message_type,
    })
    .unwrap()
  }
}
