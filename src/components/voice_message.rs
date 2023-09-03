use stylist::{self, style};
use yew::prelude::*;

use crate::{utils::style, model::Message};

#[derive(Properties, PartialEq)]
pub struct Props {
  pub message: Message,
}

#[function_component]
pub fn VoiceMessage(props: &Props) -> Html {
  let class_name = get_class_name();
  html! {
    <div class={class_name}>
    </div>
  }
}

#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(
    style!(
      r#"
        background: var(--theme-color);
        border-radius: var(--radius);
        display: inline-block;
        block-size: 40px;
        inline-size: 40px;
        border: 1px solid rgba(255, 255, 255, 0.1);
    "#
    )
  )
}
