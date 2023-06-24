use bounce::Atom;
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
