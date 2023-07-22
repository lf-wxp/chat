use bounce::Atom;
use std::fmt::{self, Display};

#[derive(Atom, PartialEq, Clone)]
pub struct Conversation(pub String);

impl Default for Conversation {
  fn default() -> Self {
    Conversation("".to_string())
  }
}

impl Display for Conversation {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}
