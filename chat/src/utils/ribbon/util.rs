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

pub struct RibbonSet {
  pub vertical_position: Position,
  pub horizontal_speed: f64,
  pub ribbon_count: usize,
  pub stroke_size: f64,
  pub parallax_amount: f64,
  pub animate_sections: bool,
  pub scroll: f64,
}

#[derive(Debug, PartialEq, Default, Copy, Clone)]
pub enum Dir {
  #[default]
  Right,
  Left,
}
