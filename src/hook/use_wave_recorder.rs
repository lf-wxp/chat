use std::rc::Rc;

use bounce::use_atom_value;
use gloo_console::log;
use wasm_bindgen::JsValue;
use web_sys::{Blob, HtmlCanvasElement};
use yew::prelude::*;

use crate::{
  store::{Theme, ThemeColor},
  utils::{VisualizeColor, WaveRecorder},
};

#[hook]
pub fn use_wave_recorder() -> (
  NodeRef,
  Rc<dyn Fn()>,
  Rc<dyn Fn() -> Box<dyn futures::Future<Output = Result<Blob, JsValue>>>>,
) {
  let canvas_node_ref = use_node_ref();
  let theme = use_atom_value::<Theme>();
  let recorder = use_mut_ref(|| {
    let ThemeColor {
      primary_color,
      theme_color,
      ..
    } = theme.get_color();
    WaveRecorder::new(
      VisualizeColor {
        background: theme_color,
        rect_color: primary_color,
        opacity: 0.8,
      },
      None,
    )
  });

  let start = {
    let canvas = canvas_node_ref.clone();
    let recorder = recorder.clone();
    let func = move || {
      let canvas = canvas.clone();
      let recorder = recorder.clone();
      let fut = async move {
        if let Ok(recorder) = recorder.as_ref().borrow_mut().as_mut() {
          let canvas = canvas.cast::<HtmlCanvasElement>();
          recorder.set_canvas(canvas);
          recorder.start().await;
        };
      };
      wasm_bindgen_futures::spawn_local(fut);
    };
    Rc::new(func) as Rc<dyn Fn()>
  };

  let stop = {
    let recorder = recorder.clone();
    Rc::new(move || {
      let recorder = recorder.clone();
      Box::new(async move {
        if let Ok(recorder) = recorder.as_ref().borrow_mut().as_mut() {
          recorder.stop().await
        } else {
          Err(JsValue::from_str("no blob"))
        }
      }) as Box<dyn futures::Future<Output = Result<Blob, JsValue>>>
    })
  };

  (canvas_node_ref, start, stop)
}
