use bounce::{Atom, BounceStates, Selector};
use fake::{uuid::UUIDv1, Dummy, Fake, Faker};
use message::Client;
use pinyin::ToPinyin;
use serde::{Deserialize, Serialize};
use std::rc::Rc;

use crate::utils::faker::RandomName;

#[derive(PartialEq, Debug, Dummy, Clone, Atom, Serialize, Deserialize, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct User {
  #[dummy(faker = "UUIDv1")]
  pub uuid: String,
  #[dummy(faker = "RandomName")]
  pub name: String,
}

impl Default for User {
  fn default() -> Self {
    #[cfg(feature = "dev")]
    return Faker.fake::<User>();
    #[cfg(not(feature = "dev"))]
    return User {
      uuid: "".to_string(),
      name: "".to_string(),
    };
  }
}

impl From<Client> for User {
  fn from(value: Client) -> Self {
    Self {
      uuid: value.uuid,
      name: value.name,
    }
  }
}

impl From<User> for Client {
  fn from(value: User) -> Self {
    Self {
      uuid: value.uuid,
      name: value.name,
    }
  }
}

impl User {
  pub fn update_name(self, name: String) -> Self {
    Self { name, ..self }
  }
}

#[derive(Debug, Clone)]
pub struct UserGroup {
  pub letter: String,
  pub users: Vec<User>,
}

#[derive(Atom, PartialEq, Debug)]
pub struct Users(pub Vec<User>);

impl Dummy<Faker> for Users {
  fn dummy_with_rng<R: rand::Rng + ?Sized>(_: &Faker, _rng: &mut R) -> Self {
    Users((0..10).map(|_| Faker.fake::<User>()).collect::<Vec<User>>())
  }
}

impl Selector for Users {
  fn select(states: &BounceStates) -> Rc<Self> {
    let user = states.get_atom_value::<User>();
    let users = states.get_atom_value::<Users>();
    let users = users
      .0
      .clone()
      .into_iter()
      .filter(|x| *x != *user)
      .collect::<Vec<User>>();
    Rc::from(Users(users))
  }
}

impl Users {
  pub fn group_with_alphabet(&self, keyword: String) -> Vec<UserGroup> {
    let mut group = ('a'..='z')
      .map(|x| UserGroup {
        letter: x.to_string(),
        users: vec![],
      })
      .collect::<Vec<UserGroup>>();
    group.push(UserGroup {
      letter: "#".to_string(),
      users: vec![],
    });

    self
      .0
      .iter()
      .filter(|user| {
        let name = user.name.to_lowercase();
        let keyword = keyword.to_lowercase();
        let py_name = (&name[..])
          .to_pinyin()
          .map(|x| x.map_or("".to_string(), |x| x.first_letter().to_string()))
          .collect::<String>();
        name.contains(&keyword) || py_name.contains(&keyword)
      })
      .for_each(|x| {
        let User { name, uuid: _ } = x;
        let char = name.chars().next().unwrap_or('#');
        let char = if char.is_numeric() { '#' } else { char };
        let letter = match char.to_pinyin() {
          Some(letter) => letter.first_letter().to_string(),
          None => char.to_string(),
        };
        let target = group.iter_mut().find(|x| x.letter == letter.to_lowercase());
        if let Some(target) = target {
          target.users.push(x.clone());
        }
      });
    group
  }
  pub fn is_exist(&self, name: &str) -> bool {
    self.0.iter().any(|user| user.name == name)
  }
}

impl Default for Users {
  fn default() -> Self {
    #[cfg(feature = "fake")]
    return Faker.fake::<Users>();
    #[cfg(not(feature = "fake"))]
    return Users(vec![]);
  }
}
