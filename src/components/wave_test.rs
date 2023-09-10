use bounce::use_atom_value;
use stylist::{self, style};
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_icons::{Icon, IconId};

use crate::{
  model::VisualizeColor,
  store::{Theme, ThemeColor},
  utils::{get_target, read_file, style, WaveSurfer},
};

#[function_component]
pub fn WaveTest() -> Html {
  let class_name = get_class_name();
  let input_node_ref = use_node_ref();
  let theme = use_atom_value::<Theme>();
  let onclick = {
    let input_node_ref = input_node_ref.clone();
    Callback::from(move |_| {
      if let Some(input) = input_node_ref
        .get()
        .and_then(|input| input.dyn_into::<HtmlInputElement>().ok())
      {
        wasm_bindgen_futures::spawn_local(async move { input.click() });
      }
    })
  };
  let onchange = Callback::from(move |e: Event| {
    let target = get_target::<Event, HtmlInputElement>(e);
    let theme = theme.clone();
    if let Some(target) = target {
      if let Some(file) = target.files().and_then(|x| x.get(0)) {
        wasm_bindgen_futures::spawn_local(async move {
          let buffer = read_file(file).await.unwrap();
          let ThemeColor {
            primary_color,
            theme_color,
            ..
          } = theme.get_color();
          if let Ok(wave_surfer) = WaveSurfer::new(
            ".canvas_container".to_string(),
            VisualizeColor {
              background: theme_color,
              rect_color: primary_color,
              opacity: 0.8,
            },
          ) {
            let _ = wave_surfer.load_from_array_buffer(buffer).await;
          }
        });
      }
    }
  });

  html! {
    <div class={class_name}>
      <div class="canvas_container"></div>
      <input type="file" accept="*/*" class="fake-input" ref={input_node_ref} {onchange} />
      <Icon {onclick} icon_id={IconId::FontAwesomeRegularImages} class="icon" width="16px" height="16px" />
    </div>
  }
}

#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
      inline-size: 300px;
      block-size: 50px;
      position: relative;
      margin-inline-start: 10px;
      .canvas_container {
        block-size: 100%;
      }
      canvas {
        inline-size: 100%;
        block-size: 100%;
      }
      .fake-input {
        display: none;
      }
      .icon {
        cursor: pointer;
        color: #8896a4;
        transition: all 0.2s ease;
      }
      .icon:hover {
        color: #51b66d;
      }
    "#
  ))
}
