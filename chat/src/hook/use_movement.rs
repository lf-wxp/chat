use yew::prelude::*;
use yew_hooks::use_event_with_window;

#[derive(Clone, Copy, Debug)]
pub struct Movement {
  pub x: i32,
  pub y: i32,
}

#[hook]
pub fn use_movement<F>(callback: F)
where
  F: Fn(Movement) + 'static,
{
  use_event_with_window("mousemove", move |e: MouseEvent| {
    callback(Movement {
      x: e.movement_x(),
      y: e.movement_y(),
    });
  });
}
