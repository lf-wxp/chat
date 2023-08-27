use bounce::Atom;
use std::fmt::{self, Display};

pub struct ThemeColor {
  pub theme_color: String,
  pub primary_color: String,
  pub ancillary_color: String,
  pub font_color: String,
}

#[derive(Atom, PartialEq, Default)]
pub enum Theme {
  Light,
  #[default]
  Dark,
}

impl Theme {
  pub fn get_color(&self) -> ThemeColor {
    match *self {
      Theme::Dark => ThemeColor {
        theme_color: "#161c20".to_string(),
        ancillary_color: "#262d33".to_string(),
        primary_color: "#51b66d".to_string(),
        font_color: "white".to_string(),
      },
      Theme::Light => ThemeColor {
        theme_color: "#161c20".to_string(),
        primary_color: "#51b66d".to_string(),
        ancillary_color: "#262d33".to_string(),
        font_color: "white".to_string(),
      },
    }
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
