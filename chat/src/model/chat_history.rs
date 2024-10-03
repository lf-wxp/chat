use chrono::prelude::*;
use chrono::serde::ts_seconds;
use fake::{faker::chrono::zh_cn::DateTime, uuid::UUIDv1, Dummy, Fake, Faker};
use js_sys::{ArrayBuffer, Uint8Array};
use nanoid::nanoid;
use serde::{
  de::{self, Visitor},
  Deserialize, Deserializer, Serialize, Serializer,
};
use std::{collections::HashMap, fmt};

use crate::utils::faker::{FakeMessage, FakeMessageState, RandomName};

#[derive(PartialEq, Clone, Debug, Deserialize, Serialize)]
pub enum Message {
  Text(String),
  #[serde(
    serialize_with = "serialize_array_buffer",
    deserialize_with = "deserialize_array_buffer"
  )]
  Audio(ArrayBuffer),
  #[serde(
    serialize_with = "serialize_array_buffer",
    deserialize_with = "deserialize_array_buffer"
  )]
  File(ArrayBuffer),
  #[serde(
    serialize_with = "serialize_array_buffer",
    deserialize_with = "deserialize_array_buffer"
  )]
  Image(ArrayBuffer),
}

fn serialize_array_buffer<S>(buffer: &ArrayBuffer, serializer: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  let bytes = Uint8Array::new(buffer).to_vec();
  serializer.serialize_bytes(&bytes)
}

fn deserialize_array_buffer<'de, D>(deserializer: D) -> Result<ArrayBuffer, D::Error>
where
  D: Deserializer<'de>,
{
  struct ArrayBufferVisitor;

  impl<'de> Visitor<'de> for ArrayBufferVisitor {
    type Value = ArrayBuffer;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
      formatter.write_str("a Message")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
      E: de::Error,
    {
      let buffer = Uint8Array::from(v).buffer();
      Ok(buffer)
    }
  }

  deserializer.deserialize_bytes(ArrayBufferVisitor)
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

#[derive(PartialEq, Debug, Dummy, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
  #[dummy(faker = "UUIDv1")]
  pub uuid: String,
  #[dummy(faker = "RandomName")]
  pub name: String,
  #[dummy(faker = "DateTime()")]
  #[serde(with = "ts_seconds")]
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
