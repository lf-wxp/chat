use bounce::Atom;
use std::fmt::{self, Display};

#[derive(Atom, PartialEq, Default)]
pub struct Refresh(bool);

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
