use bounce::Atom;
use std::fmt::{self, Display};

#[derive(Atom, PartialEq)]
pub struct Refresh(bool);

impl Default for Refresh {
  fn default() -> Self {
    Refresh(false)
  }
}

impl Refresh {
  pub fn refresh(&self) -> Self {
    let val = self.0;
    Refresh(!val)
  }
}

impl Display for Refresh {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}
