use bounce::Atom;
use std::fmt::{self, Display};

#[derive(Debug)]
pub struct ThemeColor {
  pub theme_color: String,
  pub theme_color_rgb: String,
  pub primary_color: String,
  pub ancillary_color: String,
  pub ancillary_color_rgb: String,
  pub font_color: String,
}

impl ThemeColor {
  pub fn new() -> ThemeColor {
    ThemeColor {
      theme_color: "".to_string(),
      theme_color_rgb: "".to_string(),
      primary_color: "".to_string(),
      ancillary_color: "".to_string(),
      ancillary_color_rgb: "".to_string(),
      font_color: "".to_string(),
    }
  }
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
        theme_color_rgb: "22, 28, 32".to_string(),
        ancillary_color: "#262d33".to_string(),
        ancillary_color_rgb: "38, 45, 51".to_string(),
        primary_color: "#51b66d".to_string(),
        font_color: "white".to_string(),
      },
      Theme::Light => ThemeColor {
        theme_color: "#ffffff".to_string(),
        theme_color_rgb: "255, 255, 255".to_string(),
        ancillary_color: "#EEF0F7".to_string(),
        ancillary_color_rgb: "238, 240, 247".to_string(),
        primary_color: "#51b66d".to_string(),
        font_color: "#071525".to_string(),
      },
    }
  }

  pub fn get_css_text(&self) -> String {
    let ThemeColor {
      theme_color,
      theme_color_rgb,
      primary_color,
      ancillary_color,
      ancillary_color_rgb,
      font_color,
    } = self.get_color();
    format!("--theme-color: {theme_color};--theme-color-rgb: {theme_color_rgb};--primary-color: {primary_color};--theme-ancillary-color: {ancillary_color}; --theme-ancillary-color-rgb: {ancillary_color_rgb}; --font-color:{font_color}")
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
