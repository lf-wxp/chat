use stylist::{self, style};
use yew::prelude::*;

use crate::utils::style;

#[function_component]
pub fn NotFound() -> Html {
  let class_name = get_class_name();

  html! {
    <section class={class_name}>
    {{"notFound"}}
    </section>
  }
}

fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
    "#
  ))
}
