use stylist::{self, style};
use yew::prelude::*;

use crate::{utils::style, components::Text};

#[function_component]
pub fn Home() -> Html {
  let class_name = get_class_name();

  html! {
    <section class={class_name}>
    {{"home"}}
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
