use std::f64::consts::PI;

use wasm_bindgen::JsValue;
use web_sys::CanvasRenderingContext2d;

use super::{
  point::{Point, PointAction},
  util::{num_in_range, Dir},
};

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Section {
  pub points: [Point; 3],
  pub color: f64,
  pub delay: f64,
  pub dir: Dir,
  pub alpha: f64,
  pub phase: f64,
}

impl Section {
  fn points_update_batch(&mut self, callback: &dyn Fn(&mut Point) -> ()) -> () {
    self.points.iter_mut().for_each(callback);
  }

  fn points_update_batch_action(&mut self, action: PointAction, x: f64, y: f64) -> () {
    match action {
      PointAction::Add => {
        self.points_update_batch(&|point| point.add(x, y));
      }
      PointAction::Subtract => {
        self.points_update_batch(&|point| point.subtract(x, y));
      }
      PointAction::Multiple => {
        self.points_update_batch(&|point| point.multiple(x, y));
      }
      PointAction::Divide => {
        self.points_update_batch(&|point| point.divide(x, y));
      }
      PointAction::ClampX => {
        self.points_update_batch(&|point| point.clamp_x(x, y));
      }
      PointAction::ClampY => {
        self.points_update_batch(&|point| point.clamp_y(x, y));
      }
      PointAction::FlipX => {
        self.points_update_batch(&|point| point.flip_x());
      }
      PointAction::FlipY => {
        self.points_update_batch(&|point| point.flip_y());
      }
      _ => {
        self.points_update_batch(&|point| point.set(x, y));
      }
    }
  }

  pub fn is_done(&self) -> bool {
    self.phase >= 1.0 && self.alpha <= 0.0
  }

  pub fn update_section(&mut self, is_animate: bool) -> () {
    if self.delay <= 0.0 {
      self.phase += 0.02;
      self.alpha = self.phase.sin();
      self.alpha = num_in_range(0.0, 1.0, self.alpha);

      if is_animate {
        let mod_num = (1.0 + self.phase * PI / 2.0).sin() * 0.1;
        match self.dir {
          Dir::Right => {
            self.points_update_batch_action(PointAction::Add, mod_num, 0.0);
          }
          _ => {
            self.points_update_batch_action(PointAction::Subtract, mod_num, 0.0);
          }
        }
        self.points_update_batch_action(PointAction::Add, 0.0, mod_num);
      }
    } else {
      self.delay -= 0.5;
    }
  }

  pub fn get_points(&self) -> [Point; 3] {
    self.points.clone()
  }

  pub fn get_style(&self, saturation: String, brightness: String) -> String {
    format!(
      "hsla({}, {}, {}, {})",
      self.color, saturation, brightness, self.alpha
    )
  }

  pub fn draw(
    &self,
    ctx: &CanvasRenderingContext2d,
    parallax_amount: f64,
    scroll: f64,
    stroke_size: f64,
    saturation: String,
    brightness: String,
  ) -> () {
    let points = self.get_points();
    let style = self.get_style(saturation, brightness);
    ctx.save();
    if parallax_amount != 0.0 {
      ctx.translate(0.0, scroll * parallax_amount).unwrap();
    }
    ctx.begin_path();
    let [point_1, point_2, point_3] = points;
    ctx.move_to(point_1.x, point_1.y);
    ctx.line_to(point_2.x, point_2.y);
    ctx.line_to(point_3.x, point_3.y);
    ctx.set_fill_style(&JsValue::from_str(&style));
    ctx.fill();

    if stroke_size > 0.0 {
      ctx.set_line_width(stroke_size);
      ctx.set_stroke_style(&JsValue::from_str(&style));
      ctx.set_line_cap("round");
      ctx.stroke();
    }
    ctx.restore();
  }
}
