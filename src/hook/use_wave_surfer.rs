use std::pin::Pin;
use std::rc::Rc;
use std::cell::RefCell;

use bounce::use_atom_value;
use futures::Future;
use js_sys::ArrayBuffer;
use wasm_bindgen::JsValue;
use yew::prelude::*;

use crate::{
  model::VisualizeColor,
  store::{Theme, ThemeColor},
  utils::WaveSurfer,
};

type StartAction = dyn Fn() + 'static;
type StopAction = dyn Fn() + 'static;
type LoadAction = dyn Fn(ArrayBuffer) -> Pin<Box<dyn Future<Output = Result<(), JsValue>> + 'static>> + 'static;

#[hook]
pub fn use_wave_surfer() -> (NodeRef, Rc<StartAction>, Rc<StopAction>, Rc<LoadAction>) {
  let wrap_node_ref = use_node_ref();
  let theme = use_atom_value::<Theme>();
  let wave: Rc<RefCell<Option<WaveSurfer>>> = use_mut_ref(Default::default);

  let start = {
    let wave = wave.clone();
    Rc::new(move || {
      if let Some(wave) = wave.borrow_mut().as_mut() {
        let _ = wave.start();
      };
    }) as Rc<StartAction>
  };

  let stop = {
    let wave = wave.clone();
    Rc::new(move || {
      if let Some(wave) = wave.borrow_mut().as_mut() {
        let _ = wave.stop();
      };
    }) as Rc<StopAction>
  };

  let load = {
    let wave = wave.clone();
    Rc::new(move |array_buffer: ArrayBuffer| -> Pin<Box<dyn Future<Output = Result<(), JsValue>> + 'static>> {
      let wave = wave.clone();
      Box::pin(async move {
        if let Some(wave) = wave.borrow_mut().as_mut() {
          wave.load_from_array_buffer(array_buffer).await?;
        }
        Ok(())
      })
    }) as Rc<LoadAction>
  };
  let wrap = wrap_node_ref.clone();
  let wrap_clone = wrap_node_ref.clone();

  use_effect_with_deps(move |_| {
    let ThemeColor {
      primary_color,
      theme_color,
      ..
    } = theme.get_color();
    if let Some(wrap) = wrap_node_ref.clone().cast::<web_sys::Element>() {
      *wave.borrow_mut() = Some(WaveSurfer::new(
        wrap,
        VisualizeColor {
          background: theme_color,
          rect_color: primary_color,
          opacity: 0.8,
        },
      ).unwrap());
    }
    || ()
  }, wrap);

  (wrap_clone, start, stop, load)
}
