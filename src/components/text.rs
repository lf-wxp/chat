use bounce::use_atom_value;
use gloo_console::log;
use stylist::{self, style};
use web_sys::MouseEvent;
use yew::prelude::*;
use yew::Callback;

use crate::store::Theme;
use crate::utils::style;

#[function_component]
pub fn Text() -> Html {
  let onclick = Callback::from(|e: MouseEvent| {
    log!("the event is ", e);
  });

  let class_name = get_class_name();
  let theme = use_atom_value::<Theme>();

  html! {
    <>
      <h1 class={class_name}>{"theme is "}{&theme}</h1>
      <button {onclick}>{ "click hello"}</button>
    </>
  }
}

fn get_class_name() -> String {
  style::get_class_name(
    style!(
      r#"
        position: static;
        color: blue;
    "#
    )
  )
}
