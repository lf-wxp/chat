use bounce::Atom;
use chrono::prelude::*;
use fake::{
  faker::chrono::zh_cn::DateTime,
  uuid::UUIDv1,
  Dummy, Fake, Faker,
};
use std::collections::HashMap;

use crate::{
  model,
  utils::faker::{FakeMessage, RandomName},
};

#[derive(PartialEq, Debug, Dummy, Clone)]
pub struct Message {
  #[dummy(faker = "UUIDv1")]
  pub uuid: String,
  #[dummy(faker = "RandomName")]
  pub name: String,
  #[dummy(faker = "DateTime()")]
  pub time: DateTime<Utc>,
  #[dummy(faker = "FakeMessage")]
  pub message: model::Message,
}

#[derive(Atom, PartialEq, Dummy, Debug)]
pub struct MessageBunch(pub HashMap<String, Vec<Message>>);

impl Default for MessageBunch {
  fn default() -> Self {
    #[cfg(feature = "fake")]
    return Faker.fake::<MessageBunch>();
    #[cfg(not(feature = "fake"))]
    return MessageBunch(HashMap::new());
  }
}
