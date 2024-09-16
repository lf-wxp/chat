use bounce::Atom;
use fake::{uuid::UUIDv1, Dummy, Fake, Faker};

use crate::utils::faker::{FakeUsers, RandomName};
use super::User;

#[derive(PartialEq, Clone, Dummy, Atom, Default)]
pub struct ChatGroup {
  #[dummy(faker = "UUIDv1")]
  pub id: String,
  #[dummy(faker = "RandomName")]
  pub name: String,
  #[dummy(faker = "FakeUsers")]
  pub users: Vec<User>,
}

#[derive(Atom, PartialEq, Clone, Dummy)]
pub struct ChatGroups(pub Vec<ChatGroup>);

impl Dummy<ChatGroups> for ChatGroups {
  fn dummy_with_rng<R: rand::Rng + ?Sized>(_config: &ChatGroups, _rng: &mut R) -> Self {
    ChatGroups(
      (0..10)
        .map(|_| Faker.fake::<ChatGroup>())
        .collect::<Vec<ChatGroup>>(),
    )
  }
}

impl Default for ChatGroups {
  fn default() -> Self {
    #[cfg(feature = "dev")]
    return Faker.fake::<ChatGroups>();
    #[cfg(not(feature = "dev"))]
    return ChatGroups::default()
  }
}
