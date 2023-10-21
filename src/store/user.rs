use bounce::Atom;
use fake::{uuid::UUIDv1, Dummy, Fake, Faker};
use pinyin::ToPinyin;

use crate::utils::faker::RandomName;

#[derive(PartialEq, Debug, Dummy, Clone, Atom)]
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
    return User { uuid: "".to_string(), name: "".to_string() };
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
    #[cfg(feature = "dev")]
    return Faker.fake::<Users>();
    #[cfg(not(feature = "dev"))]
    return Users(vec![]);
  }
}
