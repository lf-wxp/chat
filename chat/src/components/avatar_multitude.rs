use stylist::{self, style};
use yew::prelude::*;

use crate::{components::Avatar, utils::style};

#[derive(Properties, PartialEq)]
pub struct Props {
  pub names: Vec<String>,
}

#[function_component]
pub fn AvatarMultitude(props: &Props) -> Html {
  let class_name = get_class_name();
  html! {
    <section class={class_name}>
      { for props.names.clone().into_iter().map(|name| html! {
        <Avatar name={name} />
      })}
    </section>
  }
}
#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
        background: var(--theme-color);
        border-radius: var(--radius);
        display: inline-block;
        block-size: var(--avatar-size, 40px);
        inline-size: var(--avatar-size, 40px);
        border: 1px solid rgba(255, 255, 255, 0.1);
        flex: 0 0 auto;
    "#
  ))
}
