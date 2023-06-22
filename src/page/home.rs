use stylist::{self, style};
use yew::prelude::*;

use crate::{utils::style, components::{Text, Avatar}};

#[function_component]
pub fn Home() -> Html {
  let class_name = get_class_name();

  html! {
    <section class={class_name}>
    {{"home"}}
      <Avatar name={"user"} />
      <Avatar name={"master"} />
      <Avatar name={"test"} />
      <Text />
    </section>
  }
}

fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
    "#
  ))
}
