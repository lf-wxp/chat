use bounce::{Atom, Slice};
use fake::{uuid::UUIDv1, Dummy, Fake, Faker};
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use yew::Reducible;

use crate::utils::faker::{FakeUsers, RandomName};

use super::User;
#[derive(Clone, Dummy, Atom, Default, Debug, Serialize, Deserialize)]
pub struct Chat {
  #[dummy(faker = "UUIDv1")]
  pub id: String,
  #[dummy(faker = "RandomName")]
  pub name: String,
  #[dummy(faker = "FakeUsers")]
  pub users: Vec<User>,
}

impl Chat {
  pub fn filter(&self, keyword: &str) -> bool {
    self.name.contains(keyword)
  }

  pub fn new(users: Vec<User>, name: Option<&str>) -> Self {
    let binding = users
      .iter()
      .map(|x| x.name.clone())
      .collect::<Vec<String>>()
      .join("、");
    let name = name.unwrap_or(&binding);
    Chat {
      id: nanoid!(),
      users,
      name: name.to_string(),
    }
  }

  pub fn update_name(&mut self, user: &User) {
    let names = self
      .users
      .iter()
      .filter(|x| **x != *user)
      .map(|x| x.name.clone())
      .collect::<Vec<String>>();
    self.name = if names.len() == 1 {
      names[0].clone()
    } else {
      names.join("、")
    }
  }
}

impl PartialEq for Chat {
  fn eq(&self, other: &Self) -> bool {
    self.users == other.users
  }
}

#[derive(PartialEq, Clone)]
pub enum ChatsAction {
  Append(Chat),
}
#[derive(Slice, Atom, PartialEq, Clone, Debug)]
pub struct Chats(pub Vec<Chat>);

impl Dummy<Faker> for Chats {
  fn dummy_with_rng<R: rand::Rng + ?Sized>(_: &Faker, _rng: &mut R) -> Self {
    let chats = (0..3).map(|_| Faker.fake::<Chat>()).collect::<Vec<Chat>>();
    Chats(chats)
  }
}

impl Chats {
  pub fn find(&self, chat: &Chat) -> bool {
    let chats = self.0.clone();
    chats.iter().any(|x| *x == *chat)
  }
}

impl Reducible for Chats {
  type Action = ChatsAction;
  fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
    match action {
      ChatsAction::Append(chat) => {
        let mut chats = self.0.clone();
        if !self.find(&chat) {
          chats.push(chat);
          return Self(chats).into();
        }
        self
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
      return &chat.id;
    }
    ""
  }
  pub fn name(&self) -> &str {
    if let Some(chat) = &self.0 {
      return &chat.name;
    }
    ""
  }

  pub fn remote_client_ids(&self, user_uuid: &str) -> Vec<String> {
    if let Some(chat) = &self.0 {
      return chat
        .users
        .iter()
        .map(|x| x.uuid.clone())
        .filter(|x| x != user_uuid)
        .collect();
    }
    vec![]
  }
}

impl Dummy<CurrentChat> for CurrentChat {
  fn dummy_with_rng<R: rand::Rng + ?Sized>(_config: &CurrentChat, _rng: &mut R) -> Self {
    CurrentChat(Some(Chat::new(vec![Faker.fake::<User>()], None)))
  }
}
