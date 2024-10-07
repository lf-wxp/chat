use message::{MediaMessage, MessageType, ResponseMessage, ResponseMessageData};

use super::{UnicastExecute, UnicastMessage};

impl UnicastExecute for MediaMessage {}
impl UnicastMessage for MediaMessage {
  fn get_to(&self) -> String {
    self.to.clone()
  }
  fn get_message(&self, session_id: String, message_type: MessageType) -> Vec<u8> {
    bincode::serialize(&ResponseMessage {
      session_id,
      message: ResponseMessageData::Media(self.clone()),
      message_type,
    })
    .unwrap()
  }
}
