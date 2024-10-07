use chrono::prelude::*;
use chrono::serde::ts_seconds;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Clone, Debug, Deserialize, Serialize)]
pub enum Message {
  Text(String),
  Audio(Vec<u8>),
  File(Vec<u8>),
  Image(Vec<u8>),
}
#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum MessageState {
  Pending,
  Success,
  Fail,
}

#[derive(PartialEq, Clone)]
pub enum MessageAlignment {
  Left,
  Right,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Information {
  pub uuid: String,
  pub name: String,
  #[serde(with = "ts_seconds")]
  pub time: DateTime<Utc>,
  pub message: Message,
  pub state: MessageState,
}

impl Information {
  pub fn new(name: String, message: Message) -> Self {
    let uuid = nanoid!();
    let time = Utc::now();
    Information {
      uuid,
      name,
      time,
      message,
      state: MessageState::Pending,
    }
  }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
  pub from: String,
  pub to: Vec<String>,
  pub message: Information,
  pub chat: Chat,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize, Eq, Hash)]
pub struct User {
  pub uuid: String,
  pub name: String,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Chat {
  pub id: String,
  pub name: String,
  pub users: Vec<User>,
}
