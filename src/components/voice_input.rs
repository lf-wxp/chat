use gloo_console::log;
use js_sys::ArrayBuffer;
use stylist::{self, style};
use wasm_bindgen::JsCast;
use web_sys::{Blob, File, HtmlInputElement};
use yew::prelude::*;
use yew_icons::{Icon, IconId};

use crate::{
  components::{use_notify, NoticeTag},
  utils::{get_target, read_file, style, wave_recorder},
  hook::use_wave_recorder,
};

#[derive(Properties, PartialEq)]
pub struct Props {
  pub onchange: Callback<Blob>,
}

#[function_component]
pub fn VoiceInput(props: &Props) -> Html {
  let class_name = get_class_name();
  let visible = use_state(|| false);
  let (canvas_node_ref, start, end) = use_wave_recorder();

  let onclick = {
    let visible = visible.clone();
    Callback::from(move |_| {
      visible.set(!*visible);
    })
  };

  html! {
    <div class={class_name}>
      if *visible {
        <div class="popup">
          <canvas ref={canvas_node_ref} width="200" height="40"/>
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
        position: absolute;
        background: rgba(var(--theme-color-rgb, 0.7));
        border-radius: calc(var(--radius) / 2);
        transform: translateX(-50%) translateY(calc(-100% - 10px));
        left: 0;
      }
      .popup canvas {
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
        border-left: var(--triangle-size) solid transparent;
        border-right: var(--triangle-size) solid transparent;
        border-top: var(--triangle-size) solid rgba(var(--theme-color-rgb), 0.7);
      }
    "#
  ))
}
