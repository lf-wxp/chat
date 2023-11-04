use bounce::Atom;
use std::fmt::{self, Display};

#[derive(Atom, PartialEq, Clone)]
pub struct Chat(pub String);

impl Default for Chat {
  fn default() -> Self {
    Chat("".to_string())
  }
}

impl Display for Chat {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}
