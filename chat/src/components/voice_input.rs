use js_sys::ArrayBuffer;
use stylist::{self, style};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_icons::{Icon, IconId};

use crate::{
  hook::use_wave_recorder,
  utils::{blob_to_array_buffer, get_dpr, style},
};

#[derive(Properties, PartialEq)]
pub struct Props {
  pub onchange: Callback<ArrayBuffer>,
}

#[function_component]
pub fn VoiceInput(props: &Props) -> Html {
  let class_name = get_class_name();
  let visible = use_state(|| false);
  let (canvas_node_ref, start, end) = use_wave_recorder();

  let get_buffer = {
    let onchange = props.onchange.clone();
    move |buffer: ArrayBuffer| {
      onchange.emit(buffer);
    }
  };

  let onclick = {
    let visible = visible.clone();
    let get_buffer = get_buffer.clone();
    Callback::from(move |_| {
      let val = !*visible;
      visible.set(val);
      if val {
        start();
      } else {
        let end = end.clone();
        let get_buffer = get_buffer.clone();
        let future = async move {
          if let Ok(blob) = end().await {
            let buffer = blob_to_array_buffer(&blob).await.unwrap();
            get_buffer(buffer);
          }
        };
        spawn_local(future);
      }
    })
  };

  let width = format!("{}", get_dpr() * 200.0);
  let height = format!("{}", get_dpr() * 40.0);

  html! {
    <div class={class_name}>
      if *visible {
        <div class="popup">
          <canvas ref={canvas_node_ref} {width} {height} />
        </div>
      }
      <Icon {onclick} icon_id={IconId::HeroiconsSolidMicrophone} class="icon" width="16px" height="16px" />
    </div>
  }
}

#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
      inline-size: 16px;
      block-size: 16px;
      position: relative;
      margin-inline-start: 10px;
      --triangle-size: 5px;
      .icon {
        cursor: pointer;
        color: #8896a4;
        transition: all 0.2s ease;
      }
      .icon:hover {
        color: #51b66d;
      }
      .popup {
        inline-size: 200px;
        block-size: 40px;
        padding: 5px;
        position: absolute;
        background: rgba(var(--theme-color-rgb, 0.7));
        border-radius: calc(var(--radius) / 2);
        transform: translateX(-50%) translateY(calc(-100% - 10px));
        left: 0;
      }
      .popup canvas {
        width: 100%;
        height: 100%;
        overflow: hidden;
        border-radius: calc(var(--radius) / 2);
      }
      .popup::after {
        content: "";
        position: absolute;
        inset-inline-start: 0;
        inset-inline-end: 0;
        inset-block-end: calc(-1 * var(--triangle-size));
        margin: auto;
        block-size: 0;
        inline-size: 0;
        transform: translateX(50%);
        border-inline-start: var(--triangle-size) solid transparent;
        border-inline-end: var(--triangle-size) solid transparent;
        border-block-start: var(--triangle-size) solid rgba(var(--theme-color-rgb), 0.7);
      }
    "#
  ))
}
