use js_sys::ArrayBuffer;
use stylist::{self, style};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::utils::{array_buffer_to_blob_url, style};

#[derive(Properties, PartialEq)]
pub struct Props {
  pub message: ArrayBuffer,
}

#[function_component]
pub fn ImageMessage(props: &Props) -> Html {
  let class_name = get_class_name();
  let message = props.message.clone();
  let src = use_state(|| "".to_string());
  let message_clone = message.clone();

  let src_clone = src.clone();
  use_effect_with(message_clone, move |_| {
    let src_clone = src_clone.clone();
    spawn_local(async move {
      let url = array_buffer_to_blob_url(&message, "").unwrap_or("".to_string());
      src_clone.set(url);
    });
  });

  html! {
    <img class={class_name} src={(*src).clone()} />
  }
}

#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
        max-inline-size: 100%;
        border-radius: var(--radius);
    "#
  ))
}
