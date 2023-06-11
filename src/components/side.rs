use gloo_console::log;
use stylist::{self, style};
use web_sys::MouseEvent;
use yew::prelude::*;
use yew::Callback;
use yew_icons::{Icon, IconId};

use crate::utils::style;

#[function_component]
pub fn Side() -> Html {
  let class_name = get_class_name();

  html! {
    <side class={class_name}>
      <Icon icon_id={IconId::HeroiconsSolidUserGroup} class={"icon"}/>
    </side>
  }
}

fn get_class_name() -> String {
  style::get_class_name(
    style!(
      // A CSS string literal
      r#"
        .icon {
          color: white;
        }
    "#
    )
  )
}
