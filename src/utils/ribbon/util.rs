#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Position {
  Top,
  Middle,
  Bottom,
  Random,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ColorSet {
  pub saturation: String,
  pub brightness: String,
  pub alpha: f64,
  pub cycle_speed: f64,
}

#[derive(Debug, PartialEq, Default, Copy, Clone)]
pub enum Dir {
  #[default]
  Right,
  Left,
}
