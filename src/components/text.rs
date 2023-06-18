use bounce::use_atom;
use bounce::use_atom_value;
use gloo_console::log;
use stylist::{self, style};
use web_sys::MouseEvent;
use yew::prelude::*;
use yew::Callback;

use crate::store::Theme;
use crate::store::Volume;
use crate::utils::style;

#[function_component]
pub fn Text() -> Html {
  let class_name = get_class_name();
  let theme = use_atom_value::<Theme>();
  let theme_set = use_atom::<Theme>();
  let volume_value = use_atom_value::<Volume>();

  let onclick = Callback::from(move |e: MouseEvent| {
    theme_set.set(Theme::Light);
    log!("the event is ", e);
  });


  html! {
    <>
      <h1 class={class_name}>{"theme is "}{&theme}</h1>
      <h1>{"volume is "}{&volume_value.0}</h1>
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
