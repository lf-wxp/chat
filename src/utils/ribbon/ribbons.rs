use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, Event, HtmlCanvasElement};

use crate::utils::{get_window, Timer};

use super::{
  ribbon_item::Ribbon,
  util::{ColorSet, RibbonSet},
};
pub struct Ribbons {
  canvas: HtmlCanvasElement,
  color_set: ColorSet,
  ribbon_set: RibbonSet,
  ribbons: Vec<Ribbon>,
  timer: Timer,
  this: Option<Rc<RefCell<Self>>>,
}

impl Ribbons {
  pub fn new(
    canvas: HtmlCanvasElement,
    color_set: ColorSet,
    ribbon_set: RibbonSet,
  ) -> Rc<RefCell<Self>> {
    let ribbons = Rc::new(RefCell::new(Ribbons {
      canvas,
      color_set,
      ribbon_set,
      timer: Timer::new(),
      ribbons: vec![],
      this: None,
    }));
    ribbons.borrow_mut().this = Some(ribbons.clone());
    ribbons.borrow().init();
    ribbons
  }

  pub fn resize_stage(&self) {
    let container = self.canvas.parent_element();
    let (width, height) =
      container.map_or_else(|| (0, 0), |e| (e.client_width(), e.client_height()));
    self.canvas.set_width(width as u32);
    self.canvas.set_height(height as u32);
  }

  fn get_ctx(&self) -> Result<CanvasRenderingContext2d, JsValue> {
    let ctx = self
      .canvas
      .get_context("2d")?
      .ok_or("")?
      .dyn_into::<web_sys::CanvasRenderingContext2d>()
      .ok()
      .ok_or("")?;
    ctx.set_global_alpha(self.color_set.alpha);
    Ok(ctx)
  }

  fn get_size(&self) -> (f64, f64) {
    (
      self.canvas.client_width() as f64,
      self.canvas.client_height() as f64,
    )
  }

  pub fn add_ribbon(&mut self) {
    let (width, height) = self.get_size();
    let RibbonSet {
      vertical_position,
      horizontal_speed,
      ..
    } = self.ribbon_set;
    let ColorSet { cycle_speed, .. } = self.color_set;
    let ribbon = Ribbon::new(
      width,
      height,
      vertical_position,
      100_f64,
      cycle_speed,
      horizontal_speed,
    );

    self.ribbons.push(ribbon);
  }

  fn clear_finished_ribbon(&mut self) {
    let ribbons: Vec<Ribbon> = self
      .ribbons
      .clone()
      .into_iter()
      .filter(|ribbon| !ribbon.is_done)
      .collect();
    self.ribbons = ribbons;
  }

  pub fn draw(&mut self) {
    if let Ok(ctx) = self.get_ctx() {
      self.clear_rect();
      self.clear_finished_ribbon();
      let RibbonSet {
        animate_sections,
        parallax_amount,
        scroll,
        stroke_size,
        ribbon_count,
        ..
      } = self.ribbon_set;
      let ColorSet {
        saturation,
        brightness,
        ..
      } = &self.color_set;
      self.ribbons.iter_mut().for_each(|ribbon| {
        let mut done_num = 0;
        ribbon.sections.iter_mut().for_each(|section| {
          if section.is_done() {
            done_num += 1;
            return;
          }
          section.update_section(animate_sections);
          section.draw(
            &ctx,
            parallax_amount,
            scroll,
            stroke_size,
            saturation.clone(),
            brightness.clone(),
          );
        });
        if done_num >= ribbon.sections.len() {
          ribbon.set_done();
        }
      });
      if self.ribbons.len() < ribbon_count {
        self.add_ribbon();
      }
    }
  }

  fn clear_rect(&mut self) {
    if let Ok(ctx) = self.get_ctx() {
      let (width, height) = self.get_size();
      ctx.clear_rect(0.0, 0.0, width, height);
    }
  }

  pub fn bind_event(&self) {
    if let Some(ribbons) = &self.this {
      let ribbons = ribbons.clone();
      let window = get_window();
      let closure = Closure::<dyn Fn(_)>::new(move |_: Event| {
        ribbons.borrow().resize_stage();
      });
      window
        .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())
        .ok();
      closure.forget();
    }
  }

  fn subscribe(&self) {
    if let Some(ribbons) = &self.this {
      let ribbons = ribbons.clone();
      self.timer.subscribe(move || {
        ribbons.borrow_mut().draw();
      });
    }
  }

  pub fn init(&self) {
    self.resize_stage();
    self.bind_event();
    self.subscribe();
    self.timer.start();
  }
}
