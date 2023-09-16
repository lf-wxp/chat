use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;

use super::request_animation_frame;

type TimerCallbacks = Rc<RefCell<Vec<Rc<RefCell<dyn FnMut()>>>>>;
type RecursiveCallback = Rc<RefCell<Option<Closure<dyn FnMut()>>>>;

pub struct Timer {
  callbacks: TimerCallbacks,
  is_running: Rc<RefCell<bool>>,
}

impl Timer {
  pub fn new() -> Self {
    Timer {
      callbacks: Rc::new(RefCell::new(Vec::new())),
      is_running: Rc::new(RefCell::new(false)),
    }
  }

  pub fn subscribe<F: 'static + FnMut()>(&self, callback: F) {
    self
      .callbacks
      .borrow_mut()
      .push(Rc::new(RefCell::new(callback)));
  }

  pub fn start(&self) {
    if !*self.is_running.borrow() {
      *self.is_running.borrow_mut() = true;
      self.run();
    }
  }

  pub fn stop(&self) {
    *self.is_running.borrow_mut() = false;
  }

  pub fn destroy(&self) {
    self.stop();
  }

  fn run(&self) {
    let callbacks_clone = self.callbacks.clone();
    let is_running_clone = self.is_running.clone();

    let f: RecursiveCallback = Rc::new(RefCell::new(None));
    let g = f.clone();

    *g.borrow_mut() = Some(Closure::new(move || {
      if *is_running_clone.borrow() {
        for callback in callbacks_clone.borrow_mut().iter() {
          callback.borrow_mut()();
        }
        let closure = f.borrow();
        let closure_ref = closure.as_ref().unwrap();
        request_animation_frame(closure_ref);
      }
    }));
    let closure = g.borrow();
    let closure_ref = closure.as_ref().unwrap();
    request_animation_frame(closure_ref);
  }
}
