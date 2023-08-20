use gloo_console::log;
use js_sys::ArrayBuffer;
use stylist::{self, style};
use wasm_bindgen::JsCast;
use web_sys::{File, HtmlInputElement};
use yew::prelude::*;
use yew_icons::{Icon, IconId};

use crate::{
  components::{use_notify, NoticeTag},
  utils::{get_target, read_file, style},
};

#[derive(Properties, PartialEq)]
pub struct Props {
  pub onchange: Callback<ArrayBuffer>,
}

#[function_component]
pub fn ImageInput(props: &Props) -> Html {
  let class_name = get_class_name();
  let input_node_ref = use_node_ref();
  let notify = use_notify();

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
  let validate = move |file: &File| -> bool {
    let size = file.size();
    if (size / 1024f64) > 1024f64 {
      notify(
        "Image size limit over 1M".to_string(),
        NoticeTag::Warning,
        Some(3),
      );
      return false;
    }
    true
  };

  let onchange = {
    let change = props.onchange.clone();
    Callback::from(move |e: Event| {
      let target = get_target::<Event, HtmlInputElement>(e);
      if let Some(target) = target {
        let change = change.clone();
        let validate = validate.clone();
        if let Some(file) = target.files().and_then(|x| x.get(0)) {
          wasm_bindgen_futures::spawn_local(async move {
            let valid = validate(&file);
            if !valid {
              return;
            };
            let buffer = read_file(file).await.unwrap();
            change.emit(buffer);
          });
        }
      }
    })
  };

  html! {
    <div class={class_name}>
      <input type="file" accept="*/image" class="fake-input" ref={input_node_ref} {onchange} />
      <Icon {onclick} icon_id={IconId::FontAwesomeRegularImages} class="icon" width="16px" height="16px" />
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
