use bounce::use_atom_value;
use std::pin::Pin;
use std::rc::Rc;
use wasm_bindgen::JsValue;
use web_sys::{Blob, HtmlCanvasElement};
use yew::prelude::*;

use crate::{
  model::VisualizeColor,
  store::{Theme, ThemeColor},
  utils::WaveRecorder,
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
          recorder.borrow_mut().set_canvas(canvas);
          let _ = recorder.borrow_mut().start().await;
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
          recorder.borrow().stop().await
        } else {
          Err(JsValue::from_str("no blob"))
        }
      }) as Pin<Box<dyn futures::Future<Output = Result<Blob, JsValue>>>>
    })
  };
  {
    let recorder = recorder.clone();
    let theme = theme.clone();
    use_effect_with_deps(
      move |theme| {
        let ThemeColor {
          theme_color,
          primary_color,
          ..
        } = theme.get_color();
        if let Ok(recorder) = recorder.borrow_mut().as_mut() {
          recorder.borrow_mut().set_color(VisualizeColor {
            background: theme_color,
            rect_color: primary_color,
            opacity: 0.8,
          });
        };
      },
      theme,
    );
  }

  (canvas_node_ref, start, stop)
}
