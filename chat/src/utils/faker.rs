use fake::{
  faker::{lorem::raw::Sentence, name::raw::FirstName},
  locales::{EN, ZH_CN},
  Dummy, Fake, Faker,
};
use nanoid::nanoid;

use crate::{
  model::{Message, MessageState},
  store::User,
};

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

pub struct FakeUsers;
impl Dummy<FakeUsers> for Vec<User> {
  fn dummy_with_rng<R: rand::Rng + ?Sized>(_: &FakeUsers, _rng: &mut R) -> Self {
    (0..9).map(|_| Faker.fake::<User>()).collect::<Vec<User>>()
  }
}

pub struct FakeUser;
impl Dummy<FakeUser> for User {
  fn dummy_with_rng<R: rand::Rng + ?Sized>(_: &FakeUser, _rng: &mut R) -> Self {
    User {
      uuid: nanoid!(),
      name: FirstName(EN).fake(),
    }
  }
}
