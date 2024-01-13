use chrono::prelude::*;
use fake::{faker::chrono::zh_cn::DateTime, uuid::UUIDv1, Dummy, Faker, Fake};
use nanoid::nanoid;
use std::collections::HashMap;

use crate::{
  model::{Message, MessageState},
  utils::faker::{FakeMessage, FakeMessageState, RandomName},
};

#[derive(PartialEq, Debug, Dummy, Clone)]
pub struct ChatMessage {
  #[dummy(faker = "UUIDv1")]
  pub uuid: String,
  #[dummy(faker = "RandomName")]
  pub name: String,
  #[dummy(faker = "DateTime()")]
  pub time: DateTime<Utc>,
  #[dummy(faker = "FakeMessage")]
  pub message: Message,
  #[dummy(faker = "FakeMessageState")]
  pub state: MessageState,
}

impl ChatMessage {
  pub fn new(name: String, message: Message) -> Self {
    let uuid = nanoid!();
    let time = Utc::now();
    ChatMessage {
      uuid,
      name,
      time,
      message,
      state: MessageState::Pending,
    }
  }
}

#[derive(PartialEq, Dummy, Debug, Clone)]
pub struct ChatHistory(pub HashMap<String, Vec<ChatMessage>>);

impl Default for ChatHistory {
  fn default() -> Self {
    #[cfg(feature = "dev")]
    return Faker.fake::<ChatHistory>();
    #[cfg(not(feature = "dev"))]
    return ChatHistory(HashMap::new());
  }
}
