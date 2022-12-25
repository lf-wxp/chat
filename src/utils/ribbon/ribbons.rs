use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};
use yew::NodeRef;

use super::{
  ribbon::Ribbon,
  util::{get_window, request_animation_frame, ColorSet, Position},
};
pub struct Ribbons {
  pub canvas: Rc<NodeRef>,
  pub color_set: ColorSet,
  pub vertical_position: Position,
  pub horizontal_speed: f64,
  pub ribbon_count: usize,
  pub stroke_size: f64,
  pub parallax_amount: f64,
  pub animate_sections: bool,
  pub ribbons: Vec<Ribbon>,
  pub scroll: f64,
}

impl Ribbons {
  pub fn resize(&mut self) -> () {
    if let Some(canvas) = self.get_canvas() {
      Ribbons::resize_stage(&canvas)
    }
  }

  pub fn resize_stage(canvas: &HtmlCanvasElement) -> () {
    let container = canvas.parent_element();
    let (width, height) =
      container.map_or_else(|| (0, 0), |e| (e.client_width(), e.client_height()));
    canvas.set_width(width as u32);
    canvas.set_height(height as u32);
  }

  fn get_ctx(&self) -> Result<CanvasRenderingContext2d, JsValue> {
    let canvas = self.canvas.cast::<HtmlCanvasElement>().unwrap();
    let ctx = canvas
      .get_context("2d")?
      .ok_or("")?
      .dyn_into::<web_sys::CanvasRenderingContext2d>()
      .ok()
      .ok_or("")?;
    ctx.set_global_alpha(self.color_set.alpha);
    Ok(ctx)
  }

  fn get_canvas(&self) -> Option<HtmlCanvasElement> {
    self.canvas.cast::<HtmlCanvasElement>()
  }

  fn get_size(&self) -> (f64, f64) {
    let canvas = self.canvas.cast::<HtmlCanvasElement>().unwrap();
    (canvas.client_width() as f64, canvas.client_height() as f64)
  }

  pub fn add_ribbon(&mut self) -> () {
    let (width, height) = self.get_size();
    let ribbon = Ribbon::new(
      width,
      height,
      self.vertical_position,
      100_f64,
      self.color_set.cycle_speed,
      self.horizontal_speed,
    );

    self.ribbons.push(ribbon);
  }

  fn clear_finished_ribbon(&mut self) -> () {
    let ribbons: Vec<Ribbon> = self
      .ribbons
      .clone()
      .into_iter()
      .filter(|ribbon| !ribbon.is_done)
      .collect();
    self.ribbons = ribbons;
  }

  pub fn draw(&mut self) -> () {
    if let Ok(ctx) = self.get_ctx() {
      self.clear_rect();
      self.clear_finished_ribbon();
      self.ribbons.iter_mut().for_each(|ribbon| {
        let mut done_num = 0;
        ribbon.sections.iter_mut().for_each(|section| {
          if section.is_done() {
            done_num += 1;
            return;
          }
          section.update_section(self.animate_sections);
          section.draw(
            &ctx,
            self.parallax_amount,
            self.scroll,
            self.stroke_size,
            self.color_set.saturation.clone(),
            self.color_set.brightness.clone(),
          );
        });
        if done_num >= ribbon.sections.len() {
          ribbon.set_done();
        }
      });
      if self.ribbons.len() < self.ribbon_count {
        self.add_ribbon();
      }
    }
  }

  pub fn animate_drawing(mut self) -> () {
    let f = Rc::new(RefCell::new(None));
    let g = f.clone();
    *g.borrow_mut() = Some(Closure::new(move || {
      self.draw();
      request_animation_frame(f.borrow().as_ref().unwrap());
    }));
    request_animation_frame(g.borrow().as_ref().unwrap());
  }

  fn clear_rect(&mut self) -> () {
    if let Ok(ctx) = self.get_ctx() {
      let (width, height) = self.get_size();
      ctx.clear_rect(0.0, 0.0, width, height);
    }
  }

  pub fn bind_event(&mut self) -> () {
    let window = get_window();
    if let Some(canvas) = self.get_canvas() {
      let closure = Closure::<dyn Fn(_)>::new(move |_event: web_sys::Event| {
        Ribbons::resize_stage(&canvas);
      });
      window
        .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())
        .ok();
      closure.forget();
    }
  }

  pub fn init(mut self) -> () {
    self.resize();
    self.bind_event();
    self.animate_drawing();
  }
}
