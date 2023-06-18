use std::fmt::{Display, self};
use bounce::Atom;

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
  pub value: u8,
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
