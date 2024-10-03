use bounce::{Atom, Slice};
use fake::{uuid::UUIDv1, Dummy, Fake, Faker};
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use yew::Reducible;

use crate::utils::faker::{FakeUser, FakeUsers, RandomName};

use super::User;

#[derive(PartialEq, Clone, Default, Dummy, Debug, Serialize, Deserialize)]
pub struct ChatSingle {
  #[dummy(faker = "UUIDv1")]
  pub id: String,
  #[dummy(faker = "FakeUser")]
  pub user: User,
}

#[derive(PartialEq, Clone, Dummy, Atom, Default, Debug, Serialize, Deserialize)]
pub struct ChatGroup {
  #[dummy(faker = "UUIDv1")]
  pub id: String,
  #[dummy(faker = "RandomName")]
  pub name: String,
  #[dummy(faker = "FakeUsers")]
  pub users: Vec<User>,
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum Chat {
  Single(ChatSingle),
  Group(ChatGroup),
}

impl Chat {
  pub fn filter(&self, keyword: &str) -> bool {
    match self {
      Chat::Single(chat_single) => chat_single.user.name.contains(keyword),
      Chat::Group(chat_group) => chat_group.name.contains(keyword),
    }
  }

  pub fn single(user: User) -> Self {
    Chat::Single(ChatSingle {
      id: nanoid!(),
      user,
    })
  }

  pub fn group(users: Vec<User>) -> Self {
    let name = users
      .iter()
      .map(|x| x.name.clone())
      .collect::<Vec<String>>()
      .join("、");
    Chat::Group(ChatGroup {
      id: nanoid!(),
      users,
      name,
    })
  }
}

#[derive(PartialEq, Clone)]
pub enum ChatsAction {
  Append(Chat),
}
#[derive(Slice, Atom, PartialEq, Clone)]
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

impl Reducible for Chats {
  type Action = ChatsAction;
  fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
    match action {
      ChatsAction::Append(chat) => {
        let mut chats = self.0.clone();
        chats.push(chat);
        Self(chats).into()
      }
    }
  }
}

impl Default for Chats {
  fn default() -> Self {
    #[cfg(feature = "fake")]
    return Faker.fake::<Chats>();
    #[cfg(not(feature = "fake"))]
    return Chats(vec![]);
  }
}

#[derive(Atom, PartialEq, Clone, Default, Debug)]
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
  pub fn name(&self) -> &str {
    if let Some(chat) = &self.0 {
      return match chat {
        Chat::Single(chat_single) => &chat_single.user.name,
        Chat::Group(chat_group) => &chat_group.name,
      };
    }
    ""
  }

  pub fn remote_client_ids(&self) -> Vec<String> {
    if let Some(chat) = &self.0 {
      return match chat {
        Chat::Single(chat_single) => vec![chat_single.user.uuid.clone()],
        Chat::Group(chat_group) => chat_group.users.iter().map(|x| x.uuid.clone()).collect(),
      };
    }
    vec![]
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
