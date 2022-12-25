use std::ops::Range;

use rand::{self, Rng};
use wasm_bindgen::{prelude::Closure, JsCast};

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

pub fn random(rang: Range<u16>) -> u16 {
  rand::thread_rng().gen_range(rang)
}

pub fn num_in_range(start: f64, end: f64, num: f64) -> f64 {
  if num <= start {
    return start;
  }
  if num >= end {
    return end;
  }
  num
}

pub fn get_window() -> web_sys::Window {
  web_sys::window().expect("no global `window` exists")
}

pub fn request_animation_frame(f: &Closure<dyn FnMut()>) {
  get_window()
    .request_animation_frame(f.as_ref().unchecked_ref())
    .expect("should register `requestAnimationFrame` OK");
}
