use js_sys::ArrayBuffer;
use serde::{Deserialize, Serialize};

use crate::{
  store::Chat,
  utils::{array_buffer_to_struct, struct_to_array_buffer},
};

use super::ChatMessage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Information {
  pub message: ChatMessage,
  pub chat: Chat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChannelMessage {
  Information(Information),
  Command,
}

impl ChannelMessage {
  pub fn message(message: ChatMessage, chat: Chat) -> Self {
    ChannelMessage::Information(Information { message, chat })
  }
}

impl From<ChannelMessage> for ArrayBuffer {
  fn from(val: ChannelMessage) -> Self {
    struct_to_array_buffer(&val)
  }
}

impl From<ArrayBuffer> for ChannelMessage {
  fn from(value: ArrayBuffer) -> Self {
    array_buffer_to_struct(&value)
  }
}
