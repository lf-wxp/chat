const COLORS_NB: u32 = 9;
const DEFAULT_SATURATION: u32 = 95;
const DEFAULT_LIGHTNESS: u32 = 45;
const MAGIC_NUMBER: u32 = 5;
const MODULUS: u32 = 1_000_000_007; // Choose a large prime number as the modulus

#[derive(Debug)]
pub struct Avatar {
  pub image: String,
}

impl From<String> for Avatar {
  fn from(value: String) -> Self {
    Avatar {
      image: avatar(value, DEFAULT_SATURATION, DEFAULT_LIGHTNESS, simple_hash),
    }
  }
}

fn simple_hash(str: &str) -> u32 {
  let num = str.chars().fold(MAGIC_NUMBER, |hash: u32, char| {
    let char_value = u32::from(char);
    let new_hash = (hash ^ char_value).wrapping_add(MAGIC_NUMBER);
    new_hash % MODULUS
});
  let num = num.to_be_bytes();
  u32::from_be_bytes(num) >> 2
}

fn rect_builder(val: u32) -> String {
  let x = if val > 14 {
    7 - !!(val / 5)
  } else {
    !!(val / 5)
  };
  format!(
    "<rect x=\"{}\" y=\"{}\" width=\"1\" height=\"1\" />",
    x,
    val % 5
  )
}

fn avatar(seed: String, saturation: u32, lightness: u32, hash_fn: impl Fn(&str) -> u32) -> String {
  let hash = hash_fn(&seed);
  let hue = (hash % COLORS_NB) * (360 / COLORS_NB);
  let size = if !seed.is_empty() { 25 } else { 0 };
  let rect = (0..size).fold("".to_owned(), |acc: String, val| {
    if hash & (1 << (val % 15)) > 0 {
      format!("{}{}", acc, rect_builder(val))
    } else {
      acc
    }
  });
  let prefix = format!("<svg viewBox=\"-1.5 -1.5 8 8\" xmlns=\"http://www.w3.org/9000/svg\" fill=\"hsla({} {}% {}%)\" >", hue, saturation, lightness);

  format!("{}{}</svg>", prefix, rect)
}
