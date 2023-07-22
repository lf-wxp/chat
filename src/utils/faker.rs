use fake::{
  faker::{lorem::raw::Sentence, name::raw::FirstName},
  locales::{EN, ZH_CN},
  Dummy, Fake,
};

use crate::model::{Message, MessageState};

pub struct RandomName;
impl Dummy<RandomName> for String {
  fn dummy_with_rng<R: rand::Rng + ?Sized>(_config: &RandomName, rng: &mut R) -> Self {
    let x = rng.gen_range(0..=1);
    if x % 2 == 0 {
      FirstName(ZH_CN).fake()
    } else {
      FirstName(EN).fake()
    }
  }
}

pub struct FakeMessage;
impl Dummy<FakeMessage> for Message {
  fn dummy_with_rng<R: rand::Rng + ?Sized>(_config: &FakeMessage, _rng: &mut R) -> Self {
    Message::Text(Sentence(ZH_CN, 2..10).fake())
  }
}

pub struct FakeMessageState;
impl Dummy<FakeMessageState> for MessageState {
  fn dummy_with_rng<R: rand::Rng + ?Sized>(_config: &FakeMessageState, _rng: &mut R) -> Self {
    MessageState::Success
  }
}
