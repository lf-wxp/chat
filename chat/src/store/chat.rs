use bounce::Atom;
use fake::{uuid::UUIDv1, Dummy, Fake, Faker};
use nanoid::nanoid;

use crate::utils::faker::{FakeUser, FakeUsers, RandomName};

use super::User;

#[derive(PartialEq, Clone, Default, Dummy)]
pub struct ChatSingle {
  #[dummy(faker = "UUIDv1")]
  pub id: String,
  #[dummy(faker = "FakeUser")]
  pub user: User,
}

#[derive(PartialEq, Clone, Dummy, Atom, Default)]
pub struct ChatGroup {
  #[dummy(faker = "UUIDv1")]
  pub id: String,
  #[dummy(faker = "RandomName")]
  pub name: String,
  #[dummy(faker = "FakeUsers")]
  pub users: Vec<User>,
}

#[derive(PartialEq, Clone)]
pub enum Chat {
  Single(ChatSingle),
  Group(ChatGroup),
}

impl Chat {
  pub fn filter(&self, keyword: &str) -> bool {
    match self {
      Chat::Single(chat_single) => {
        chat_single.user.name.contains(keyword)
      }
      Chat::Group(chat_group) => {
        chat_group.name.contains(keyword)
      },
    }
  }
}

#[derive(Atom, PartialEq, Clone)]
pub struct Chats(pub Vec<Chat>);

impl Dummy<Faker> for Chats {
  fn dummy_with_rng<R: rand::Rng + ?Sized>(_: &Faker, _rng: &mut R) -> Self {
    let mut chats = (0..3)
      .map(|_| Chat::Single(Faker.fake::<ChatSingle>()))
      .collect::<Vec<Chat>>();
    chats.extend(
      (0..2)
        .map(|_| Chat::Group(Faker.fake::<ChatGroup>()))
        .collect::<Vec<Chat>>(),
    );
    Chats(chats)
  }
}

impl Default for Chats {
  fn default() -> Self {
    #[cfg(feature = "dev")]
    return Faker.fake::<Chats>();
    #[cfg(not(feature = "dev"))]
    return Chats(vec![]);
  }
}

#[derive(Atom, PartialEq, Clone, Default)]
pub struct CurrentChat(pub Option<Chat>);

impl CurrentChat {
  pub fn id(&self) -> &str {
    if let Some(chat) = &self.0 {
      return match chat {
        Chat::Single(chat_single) => &chat_single.id,
        Chat::Group(chat_group) => &chat_group.id,
      };
    }
    ""
  }
}

impl Dummy<CurrentChat> for CurrentChat {
  fn dummy_with_rng<R: rand::Rng + ?Sized>(_config: &CurrentChat, _rng: &mut R) -> Self {
    CurrentChat(Some(Chat::Single(ChatSingle {
      id: nanoid!(),
      user: Faker.fake::<User>(),
    })))
  }
}
