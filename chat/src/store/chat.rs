use bounce::Atom;
use fake::{uuid::UUIDv1, Dummy};

use crate::utils::faker::FakeUser;
use super::{ChatGroup, User};

#[derive(PartialEq, Clone, Default, Dummy)]
pub struct ChatSingle {
  #[dummy(faker = "UUIDv1")]
  pub id: String,
  #[dummy(faker = "FakeUser")]
  pub user: User,
}

#[derive(PartialEq, Clone)]
pub enum Chat {
  Single(ChatSingle),
  Group(ChatGroup),
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
