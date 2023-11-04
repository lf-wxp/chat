use bounce::Atom;

#[derive(Atom, PartialEq, Debug)]
pub struct Volume {
  pub value: i8,
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

impl Volume {
  pub fn new(value: i8, mute: bool) -> Self {
    let value = match value {
      0..=100 => value,
      d if d <= 0 => 0,
      d if d > 100 => 100,
      _ => value,
    };
    Volume { value, mute }
  }
}
