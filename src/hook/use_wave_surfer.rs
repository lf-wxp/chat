use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;

use bounce::use_atom_value;
use futures::Future;
use gloo_console::log;
use js_sys::ArrayBuffer;
use wasm_bindgen::JsValue;
use web_sys::Element;
use yew::prelude::*;

use crate::{
  model::VisualizeColor,
  store::{Theme, ThemeColor},
  utils::{WaveEvent, WaveEventCallback, WaveSurfer},
};

type BareAction = dyn Fn() + 'static;
type LoadAction =
  dyn Fn(ArrayBuffer) -> Pin<Box<dyn Future<Output = Result<(), JsValue>> + 'static>> + 'static;
type ReturnTuple = (
  NodeRef,
  UseStateHandle<f64>,
  UseStateHandle<bool>,
  Rc<BareAction>,
  Rc<BareAction>,
  Rc<LoadAction>,
);

#[hook]
pub fn use_wave_surfer() -> ReturnTuple {
  let wrap_node_ref = use_node_ref();
  let duration = use_state(|| 0f64);
  let playing = use_state(|| false);
  let theme = use_atom_value::<Theme>();
  let wave: Rc<RefCell<Option<WaveSurfer>>> = use_mut_ref(Default::default);

  let start = {
    let wave = wave.clone();
    Rc::new(move || {
      if let Some(wave) = wave.borrow_mut().as_mut() {
        let _ = wave.start();
      };
    }) as Rc<BareAction>
  };

  let stop = {
    let wave = wave.clone();
    Rc::new(move || {
      if let Some(wave) = wave.borrow_mut().as_mut() {
        let _ = wave.stop();
      };
    }) as Rc<BareAction>
  };

  let load = {
    let wave = wave.clone();
    let duration = duration.clone();
    let playing = playing.clone();
    Rc::new(move |array_buffer: ArrayBuffer| -> Pin<Box<dyn Future<Output = Result<(), JsValue>> + 'static>> {
      let wave = wave.clone();
      let duration = duration.clone();
      let playing = playing.clone();
      Box::pin(async move {
        if let Some(wave) = wave.borrow_mut().as_mut() {
          wave.load_from_array_buffer(array_buffer).await?;
          duration.set(wave.get_duration());
          wave.on(WaveEvent::PlayState, WaveEventCallback::PlayStateCallback(Rc::new(move |is_playing: bool| {
            playing.set(is_playing);
          })));
        }
        Ok(())
      })
    }) as Rc<LoadAction>
  };
  let wrap = wrap_node_ref.clone();
  let wrap_clone = wrap_node_ref.clone();

  {
    let wave = wave.clone();
    let theme = theme.clone();
    use_effect_with_deps(
      move |theme| {
        let ThemeColor { font_color, .. } = theme.get_color();
        if let Some(wave) = wave.borrow_mut().as_mut() {
          log!("theme change");
          let _ = wave.set_color(VisualizeColor {
            background: "transparent".to_string(),
            rect_color: font_color,
            opacity: 0.8,
          });
        };
      },
      theme,
    );
  }

  use_effect_with_deps(
    move |_| {
      let ThemeColor { font_color, .. } = theme.get_color();
      if let Some(wrap) = wrap_node_ref.clone().cast::<Element>() {
        *wave.borrow_mut() = WaveSurfer::new(
          wrap,
          VisualizeColor {
            background: "transparent".to_owned(),
            rect_color: font_color,
            opacity: 0.8,
          },
        )
        .ok();
      }
      || ()
    },
    wrap,
  );

  (wrap_clone, duration, playing, start, stop, load)
}
