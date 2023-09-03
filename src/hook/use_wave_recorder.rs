use std::pin::Pin;
use std::rc::Rc;

use bounce::use_atom_value;
use wasm_bindgen::JsValue;
use web_sys::{Blob, HtmlCanvasElement};
use yew::prelude::*;

use crate::{
  store::{Theme, ThemeColor},
  utils::{VisualizeColor, WaveRecorder},
};

type StartAction = Rc<dyn Fn()>;
type StopAction = Rc<dyn Fn() -> Pin<Box<dyn futures::Future<Output = Result<Blob, JsValue>>>>>;
#[hook]
pub fn use_wave_recorder() -> (NodeRef, StartAction, StopAction) {
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
        if let Ok(recorder) = recorder.borrow_mut().as_mut() {
          let canvas = canvas.cast::<HtmlCanvasElement>();
          recorder.set_canvas(canvas);
          let _ = recorder.start().await;
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
      Box::pin(async move {
        if let Ok(recorder) = recorder.borrow().as_ref() {
          recorder.stop().await
        } else {
          Err(JsValue::from_str("no blob"))
        }
      }) as Pin<Box<dyn futures::Future<Output = Result<Blob, JsValue>>>>
    })
  };

  (canvas_node_ref, start, stop)
}
