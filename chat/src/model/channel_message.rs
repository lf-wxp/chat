use js_sys::ArrayBuffer;
use serde::{Deserialize, Serialize};

use crate::{
  store::Chat,
  utils::{array_buffer_to_struct, struct_to_array_buffer},
};

use super::ChatMessage;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Information {
  message: ChatMessage,
  chat: Chat,
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

impl Into<ArrayBuffer> for ChannelMessage {
  fn into(self) -> ArrayBuffer {
    struct_to_array_buffer(&self)
  }
}

impl From<ArrayBuffer> for ChannelMessage {
  fn from(value: ArrayBuffer) -> Self {
    array_buffer_to_struct(&value)
  }
}
