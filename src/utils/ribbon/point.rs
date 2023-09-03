#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
  pub x: f64,
  pub y: f64,
}

impl Point {
  pub fn new(x: f64, y: f64) -> Point {
    Point { x, y }
  }

  pub fn set(&mut self, x: f64, y: f64) {
    self.x = x;
    self.y = y;
  }

  pub fn multiple(&mut self, x: f64, y: f64) {
    self.x *= x;
    self.y *= y;
  }

  pub fn divide(&mut self, x: f64, y: f64) {
    self.x /= x;
    self.y /= y;
  }

  pub fn add(&mut self, x: f64, y: f64) {
    self.x += x;
    self.y += y;
  }

  pub fn subtract(&mut self, x: f64, y: f64) {
    self.x -= x;
    self.y -= y;
  }

  pub fn clamp_x(&mut self, min: f64, max: f64) {
    self.x = min.max(self.x.max(max));
  }

  pub fn clamp_y(&mut self, min: f64, max: f64) {
    self.y = min.max(self.y.max(max));
  }

  pub fn flip_x(&mut self) {
    self.x *= -1.0;
  }

  pub fn flip_y(&mut self) {
    self.y *= -1.0;
  }

  pub fn copy(&mut self, point: Point) {
    let Point { x, y } = point;
    self.x = x;
    self.y = y;
  }

  pub fn from(point: Point) -> Point {
    let Point { x, y } = point;
    Point { x, y }
  }
}

pub enum PointAction {
  Set,
  Add,
  Multiple,
  Divide,
  Subtract,
  ClampX,
  ClampY,
  FlipX,
  FlipY,
  Copy,
}
