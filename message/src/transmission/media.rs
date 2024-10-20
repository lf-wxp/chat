use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MediaType {
  #[default]
  Video,
  Audio,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum MessageType {
  Request,
  Response,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MediaMessage {
  pub from_name: String,
  pub media_type: MediaType,
  pub message_type: MessageType,
  pub confirm: Option<bool>,
}

impl MediaMessage {
  pub fn new(from_name: String, media_type: MediaType, message_type: MessageType) -> Self {
    Self {
      from_name,
      media_type,
      message_type,
      confirm: None,
    }
  }
  pub fn confirm(&mut self) {
    self.confirm = Some(true);
  }
  pub fn invert(&mut self, from_name: String) {
    self.from_name = from_name;
    let message_type = match self.message_type {
      MessageType::Request => MessageType::Response,
      MessageType::Response => MessageType::Request,
    };
    self.message_type = message_type;
  }
}
