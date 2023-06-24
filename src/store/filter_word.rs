use bounce::Atom;
use std::fmt::{self, Display};

#[derive(Atom, PartialEq)]
pub struct FilterWord(pub String);

impl Default for FilterWord {
  fn default() -> Self {
    FilterWord("".to_string())
  }
}

impl Display for FilterWord {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}
