use bounce::Atom;
use fake::faker::name::raw::*;
use fake::locales::*;
use fake::{
  uuid::UUIDv1,
  Dummy, Fake, Faker,
};
use gloo_console::log;
use pinyin::{ToPinyin};
use std::fmt::{self, Display};

#[derive(Atom, PartialEq)]
pub enum Theme {
  Light,
  Dark,
}

impl Default for Theme {
  fn default() -> Self {
    Theme::Dark
  }
}

impl Display for Theme {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match *self {
      Theme::Dark => write!(f, "dark"),
      Theme::Light => write!(f, "light"),
    }
  }
}

#[derive(Atom, PartialEq, Debug)]
pub struct Volume {
  pub value: i8,
  pub mute: bool,
}

impl Default for Volume {
  fn default() -> Self {
    Volume {
      value: 50,
      mute: false,
    }
  }
}

impl Volume {
  pub fn new(value: i8, mute: bool) -> Self {
    let value = match value {
      0..=100 => value,
      d if d <= 0 => 0,
      d if d > 100 => 100,
      _ => value,
    };
    Volume { value, mute }
  }
}


#[derive(PartialEq, Debug, Dummy, Clone)]
pub struct User {
  #[dummy(faker = "UUIDv1")]
  pub uuid: String,
  pub name: String,
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
    Users(
      (0..10)
        .map(|x| {
          let name = if x % 2 == 0 {
            FirstName(ZH_CN).fake()
          } else {
            FirstName(EN).fake()
          };
          User {
            uuid: UUIDv1.fake(),
            name,
          }
        })
        .collect::<Vec<User>>(),
    )
  }
}

impl Users {
  pub fn group_with_alphabet(&self) -> Vec<UserGroup> {
    let mut group = ('a'..'z').into_iter().map(|x| UserGroup {
      letter: x.to_string(),
      users: vec![],
    }).collect::<Vec<UserGroup>>();
    group.push(UserGroup { letter: "#".to_string(), users: vec![] });

    self.0.iter().for_each(|x| {
      let User { name, uuid: _ } = x;
      let char = name.chars().next().unwrap_or('#');
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
}

impl Default for Users {
  fn default() -> Self {
    #[cfg(feature = "fake")]
    return Faker.fake::<Users>();
    #[cfg(not(feature = "fake"))]
    return Users(vec![]);
  }
}
