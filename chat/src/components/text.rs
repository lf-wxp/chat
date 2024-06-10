use bounce::{use_atom, use_atom_value};
use stylist::{self, style};
use web_sys::MouseEvent;
use yew::{prelude::*, Callback};

use crate::{
  store::{Theme, Volume},
  utils::style, components::{use_notify, NoticeTag, use_dialog},
};

#[function_component]
pub fn Text() -> Html {
  let class_name = get_class_name();
  let theme = use_atom_value::<Theme>();
  let theme_set = use_atom::<Theme>();
  let volume_value = use_atom_value::<Volume>();
  let notify = use_notify();
  let dialog = use_dialog();

  let onclick = Callback::from(move |_e: MouseEvent| {
    theme_set.set(Theme::Light);
    notify("content".to_string(), NoticeTag::Info, Some(3));
  });

  let ondialog = Callback::from(move |_e: MouseEvent| {
    dialog("header".to_string(), "content".to_string());
  });

  html! {
    <>
      <h1 class={class_name}>{"theme is "}{&theme}</h1>
      <h1>{format!("volume is {:?}", volume_value)}</h1>
      <button {onclick}>{ "click hello"}</button>
      <button onclick={ondialog}>{ "click dialog"}</button>
    </>
  }
}

#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
        position: static;
        color: blue;
    "#
  ))
}
